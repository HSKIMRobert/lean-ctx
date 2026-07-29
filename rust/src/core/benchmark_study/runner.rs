//! Study runner: orchestrates all arms across a dataset.

use super::experiment::{ArmResult, FourArmExperiment, StudyConfig};
use super::report::StudyReport;

/// Run the full benchmark study for the given datasets.
///
/// Returns a `StudyReport` containing per-dataset, per-arm results
/// with statistical analysis.
pub fn run_study(config: &StudyConfig, dataset_names: &[&str]) -> StudyReport {
    let mut experiments = Vec::new();

    for &name in dataset_names {
        let results: Vec<ArmResult> = config
            .arms
            .iter()
            .map(|arm| {
                tracing::info!(arm = %arm, dataset = name, "running arm");
                run_arm(config, name, *arm)
            })
            .collect();

        experiments.push(FourArmExperiment {
            config: config.clone(),
            dataset_name: name.to_string(),
            results,
        });
    }

    StudyReport::from_experiments(experiments)
}

fn run_arm(_config: &StudyConfig, _dataset: &str, arm: super::experiment::Arm) -> ArmResult {
    // Stub: real implementation loads dataset, calls LLM, runs sandbox
    ArmResult {
        arm,
        tasks_total: 0,
        tasks_passed: 0,
        total_input_tokens: 0,
        total_output_tokens: 0,
        total_cost_usd: 0.0,
        task_results: vec![],
    }
}
