//! Multi-config benchmark runner with repeated execution.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::config::{BenchConfig, CompressionProfile, ProfileMode};
use super::fixtures::{QualityScore, TaskFixture};

/// Result of running a single task under a single compression profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskRun {
    pub task_id: String,
    pub profile: String,
    pub raw_tokens: usize,
    pub compressed_tokens: usize,
    pub savings_pct: f64,
    pub quality: QualityScore,
    pub latency_us: u64,
}

/// Aggregated results for one profile across all tasks and repeats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProfileResult {
    pub profile: String,
    pub mode: ProfileMode,
    pub runs: Vec<TaskRun>,
    pub total_raw_tokens: usize,
    pub total_compressed_tokens: usize,
    pub avg_savings_pct: f64,
    pub tasks_passed: usize,
    pub tasks_total: usize,
    pub avg_quality_score: f64,
    pub avg_latency_us: u64,
}

/// Complete benchmark result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BenchmarkResult {
    pub profiles: Vec<ProfileResult>,
    pub repeats: u32,
    pub regression_detected: bool,
    pub regression_details: Vec<String>,
}

/// Run the full benchmark suite.
pub(crate) fn run_benchmark(tasks: &[TaskFixture], config: &BenchConfig) -> BenchmarkResult {
    let mut profile_results = Vec::new();

    for profile in &config.profiles {
        let mut all_runs = Vec::new();

        for _ in 0..config.repeats {
            for task in tasks {
                let run = execute_task(task, profile);
                all_runs.push(run);
            }
        }

        let result = aggregate_profile(&profile.name, profile.mode, &all_runs);
        profile_results.push(result);
    }

    let (regression_detected, regression_details) =
        check_regressions(&profile_results, config.regression_threshold);

    BenchmarkResult {
        profiles: profile_results,
        repeats: config.repeats,
        regression_detected,
        regression_details,
    }
}

fn execute_task(task: &TaskFixture, profile: &CompressionProfile) -> TaskRun {
    let raw_tokens = crate::core::tokens::count_tokens(&task.source_content);

    let start = Instant::now();
    let compressed = compress_for_profile(&task.source_content, &task.extension, profile.mode);
    let latency = start.elapsed();

    let compressed_tokens = crate::core::tokens::count_tokens(&compressed);
    let savings_pct = if raw_tokens > 0 {
        (1.0 - compressed_tokens as f64 / raw_tokens as f64) * 100.0
    } else {
        0.0
    };

    let quality = task.score(&compressed);

    TaskRun {
        task_id: task.id.clone(),
        profile: profile.name.clone(),
        raw_tokens,
        compressed_tokens,
        savings_pct,
        quality,
        latency_us: latency.as_micros() as u64,
    }
}

fn compress_for_profile(content: &str, ext: &str, mode: ProfileMode) -> String {
    match mode {
        ProfileMode::Stock => content.to_string(),
        ProfileMode::Standard => {
            // Standard = lightweight cleanup (whitespace normalization, blank line
            // collapsing). Never replaces content with signatures-only — that
            // destroys struct fields, error strings, and other non-signature data.
            crate::core::compressor::lightweight_cleanup(content)
        }
        ProfileMode::Aggressive => crate::core::compressor::aggressive_compress(content, Some(ext)),
    }
}

fn aggregate_profile(name: &str, mode: ProfileMode, runs: &[TaskRun]) -> ProfileResult {
    let total_raw: usize = runs.iter().map(|r| r.raw_tokens).sum();
    let total_compressed: usize = runs.iter().map(|r| r.compressed_tokens).sum();
    let avg_savings = if runs.is_empty() {
        0.0
    } else {
        runs.iter().map(|r| r.savings_pct).sum::<f64>() / runs.len() as f64
    };
    let tasks_passed = runs.iter().filter(|r| r.quality.passes()).count();
    let avg_quality = if runs.is_empty() {
        0.0
    } else {
        runs.iter().map(|r| r.quality.overall_score()).sum::<f64>() / runs.len() as f64
    };
    let avg_latency = if runs.is_empty() {
        0
    } else {
        runs.iter().map(|r| r.latency_us).sum::<u64>() / runs.len() as u64
    };

    ProfileResult {
        profile: name.to_string(),
        mode,
        runs: runs.to_vec(),
        total_raw_tokens: total_raw,
        total_compressed_tokens: total_compressed,
        avg_savings_pct: avg_savings,
        tasks_passed,
        tasks_total: runs.len(),
        avg_quality_score: avg_quality,
        avg_latency_us: avg_latency,
    }
}

fn check_regressions(results: &[ProfileResult], threshold: f64) -> (bool, Vec<String>) {
    let stock = results.iter().find(|r| r.mode == ProfileMode::Stock);
    let stock_score = stock.map_or(1.0, |s| s.avg_quality_score);

    let mut details = Vec::new();
    let mut any_regression = false;

    for result in results {
        if result.mode == ProfileMode::Stock {
            continue;
        }

        if stock_score > 0.0 {
            let ratio = result.avg_quality_score / stock_score;
            if ratio < threshold {
                any_regression = true;
                details.push(format!(
                    "{}: quality {:.1}% of stock baseline (threshold {:.0}%)",
                    result.profile,
                    ratio * 100.0,
                    threshold * 100.0
                ));
            }
        }

        let failed_tasks: Vec<_> = result
            .runs
            .iter()
            .filter(|r| !r.quality.passes())
            .map(|r| r.task_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if !failed_tasks.is_empty() {
            details.push(format!(
                "{}: {} tasks lost required signals: {}",
                result.profile,
                failed_tasks.len(),
                failed_tasks.join(", ")
            ));
        }
    }

    (any_regression, details)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::task_benchmark::fixtures::canonical_suite;

    #[test]
    fn stock_profile_preserves_all_signals() {
        let tasks = canonical_suite();
        let config = BenchConfig::single_profile(ProfileMode::Stock);
        let result = run_benchmark(&tasks, &config);

        assert_eq!(result.profiles.len(), 1);
        let stock = &result.profiles[0];
        assert_eq!(stock.tasks_passed, stock.tasks_total);
        assert!(!result.regression_detected);
    }

    #[test]
    fn all_profiles_produce_results() {
        let tasks = canonical_suite();
        let config = BenchConfig {
            repeats: 1,
            ..BenchConfig::default()
        };
        let result = run_benchmark(&tasks, &config);

        assert_eq!(result.profiles.len(), 3);
        for profile in &result.profiles {
            assert!(!profile.runs.is_empty());
            assert!(profile.total_raw_tokens > 0);
        }
    }

    #[test]
    fn compression_reduces_tokens() {
        let tasks = canonical_suite();
        let config = BenchConfig {
            repeats: 1,
            ..BenchConfig::default()
        };
        let result = run_benchmark(&tasks, &config);

        let stock = result
            .profiles
            .iter()
            .find(|p| p.mode == ProfileMode::Stock)
            .unwrap();
        let aggressive = result
            .profiles
            .iter()
            .find(|p| p.mode == ProfileMode::Aggressive)
            .unwrap();

        assert!(
            aggressive.total_compressed_tokens < stock.total_raw_tokens,
            "aggressive ({}) should use fewer tokens than stock ({})",
            aggressive.total_compressed_tokens,
            stock.total_raw_tokens
        );
    }

    #[test]
    fn regression_detection_works() {
        let profiles = vec![
            ProfileResult {
                profile: "stock".into(),
                mode: ProfileMode::Stock,
                runs: vec![],
                total_raw_tokens: 1000,
                total_compressed_tokens: 1000,
                avg_savings_pct: 0.0,
                tasks_passed: 10,
                tasks_total: 10,
                avg_quality_score: 1.0,
                avg_latency_us: 0,
            },
            ProfileResult {
                profile: "bad".into(),
                mode: ProfileMode::Aggressive,
                runs: vec![],
                total_raw_tokens: 1000,
                total_compressed_tokens: 500,
                avg_savings_pct: 50.0,
                tasks_passed: 5,
                tasks_total: 10,
                avg_quality_score: 0.5,
                avg_latency_us: 0,
            },
        ];

        let (detected, details) = check_regressions(&profiles, 0.95);
        assert!(detected);
        assert!(!details.is_empty());
    }
}
