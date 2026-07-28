use std::sync::{LazyLock, Mutex};
use std::time::Instant;

const DEFAULT_BUDGET_MS: u64 = 5000;
const WARN_THRESHOLD_PCT: f64 = 80.0;
const MAX_LATENCY_SAMPLES: usize = 4096;

#[derive(Debug, Clone, Copy)]
pub struct LatencyBudget {
    pub total_ms: u64,
    pub started_at: Instant,
}

#[derive(Debug, Clone)]
pub struct LatencyCheckpoint {
    pub phase: String,
    pub elapsed_ms: u64,
    pub budget_remaining_ms: u64,
    pub over_budget: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LatencyReport {
    pub total_elapsed_ms: u64,
    pub budget_ms: u64,
    pub within_budget: bool,
    pub phases: Vec<PhaseMetric>,
    pub slowest_phase: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhaseMetric {
    pub name: String,
    pub elapsed_ms: u64,
    pub pct_of_total: f64,
}

pub struct LatencyTracker {
    budget: LatencyBudget,
    phases: Vec<(String, u64)>,
    last_checkpoint: Instant,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LatencySummary {
    pub sample_count: usize,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub max_ms: u64,
}

static LATENCY_SAMPLES: LazyLock<Mutex<Vec<u64>>> = LazyLock::new(|| Mutex::new(Vec::new()));

impl LatencyBudget {
    pub fn new(total_ms: u64) -> Self {
        Self {
            total_ms,
            started_at: Instant::now(),
        }
    }

    pub fn default_budget() -> Self {
        Self::new(DEFAULT_BUDGET_MS)
    }

    pub fn elapsed_ms(&self) -> u64 {
        u64::try_from(self.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    pub fn remaining_ms(&self) -> u64 {
        self.total_ms.saturating_sub(self.elapsed_ms())
    }

    pub fn is_over_budget(&self) -> bool {
        self.elapsed_ms() > self.total_ms
    }

    pub fn is_warning(&self) -> bool {
        percentage(self.elapsed_ms(), self.total_ms) >= WARN_THRESHOLD_PCT
    }
}

impl LatencyTracker {
    pub fn new(budget_ms: u64) -> Self {
        let now = Instant::now();
        Self {
            budget: LatencyBudget {
                total_ms: budget_ms,
                started_at: now,
            },
            phases: Vec::new(),
            last_checkpoint: now,
        }
    }

    pub fn checkpoint(&mut self, phase: &str) -> LatencyCheckpoint {
        let now = Instant::now();
        let phase_elapsed = duration_ms(self.last_checkpoint, now);
        self.phases.push((phase.to_owned(), phase_elapsed));
        self.last_checkpoint = now;

        LatencyCheckpoint {
            phase: phase.to_owned(),
            elapsed_ms: phase_elapsed,
            budget_remaining_ms: self.budget.remaining_ms(),
            over_budget: self.budget.is_over_budget(),
        }
    }

    pub fn report(&self) -> LatencyReport {
        let total_elapsed_ms = self.budget.elapsed_ms();
        let phases = self
            .phases
            .iter()
            .map(|(name, elapsed_ms)| PhaseMetric {
                name: name.clone(),
                elapsed_ms: *elapsed_ms,
                pct_of_total: percentage(*elapsed_ms, total_elapsed_ms),
            })
            .collect();
        let slowest_phase = self
            .phases
            .iter()
            .max_by_key(|(_, elapsed_ms)| elapsed_ms)
            .map(|(name, _)| name.clone());

        LatencyReport {
            total_elapsed_ms,
            budget_ms: self.budget.total_ms,
            within_budget: total_elapsed_ms <= self.budget.total_ms,
            phases,
            slowest_phase,
        }
    }
}

pub fn format_latency_report(report: &LatencyReport) -> String {
    let status = if report.within_budget { "✓" } else { "✗" };
    let mut output = format!(
        "Latency: {}ms / {}ms budget ({:.1}%) {status}",
        report.total_elapsed_ms,
        report.budget_ms,
        percentage(report.total_elapsed_ms, report.budget_ms)
    );
    let name_width = report
        .phases
        .iter()
        .map(|phase| phase.name.len())
        .max()
        .unwrap_or(0);

    for phase in &report.phases {
        output.push_str(&format!(
            "\n  {:name_width$}: {}ms ({:.1}%)",
            phase.name, phase.elapsed_ms, phase.pct_of_total
        ));
    }
    output
}

pub fn record_latency(elapsed_ms: u64) {
    let mut samples = LATENCY_SAMPLES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if samples.len() == MAX_LATENCY_SAMPLES {
        samples.remove(0);
    }
    samples.push(elapsed_ms);
}

pub fn latency_percentile(percentile: f64) -> Option<u64> {
    let samples = LATENCY_SAMPLES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    percentile_from_samples(&samples, percentile)
}

pub fn latency_summary() -> Option<LatencySummary> {
    let samples = LATENCY_SAMPLES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if samples.is_empty() {
        return None;
    }

    Some(LatencySummary {
        sample_count: samples.len(),
        p50_ms: percentile_from_samples(&samples, 50.0)?,
        p95_ms: percentile_from_samples(&samples, 95.0)?,
        p99_ms: percentile_from_samples(&samples, 99.0)?,
        max_ms: samples.iter().copied().max()?,
    })
}

fn duration_ms(start: Instant, end: Instant) -> u64 {
    u64::try_from(end.duration_since(start).as_millis()).unwrap_or(u64::MAX)
}

fn percentage(value: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 * 100.0 / total as f64
    }
}

fn percentile_from_samples(samples: &[u64], percentile: f64) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let percentile = if percentile.is_finite() {
        percentile.clamp(0.0, 100.0)
    } else {
        0.0
    };
    let rank = (percentile / 100.0 * sorted.len() as f64).ceil() as usize;
    sorted.get(rank.saturating_sub(1)).copied()
}

#[cfg(test)]
mod tests {
    use super::{
        LATENCY_SAMPLES, LatencyBudget, LatencyReport, LatencyTracker, PhaseMetric,
        format_latency_report, latency_percentile, latency_summary, record_latency,
    };
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;

    static SAMPLE_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn clear_samples() {
        LATENCY_SAMPLES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    #[test]
    fn test_budget_creation() {
        let budget = LatencyBudget::new(1234);
        assert_eq!(budget.total_ms, 1234);
    }

    #[test]
    fn test_default_budget_creation() {
        assert_eq!(LatencyBudget::default_budget().total_ms, 5000);
    }

    #[test]
    fn test_budget_remaining_decreases() {
        let budget = LatencyBudget::new(10_000);
        thread::sleep(Duration::from_millis(10));
        assert!(budget.remaining_ms() < budget.total_ms);
    }

    #[test]
    fn test_budget_over_budget() {
        let budget = LatencyBudget::new(1);
        thread::sleep(Duration::from_millis(10));
        assert!(budget.is_over_budget());
    }

    #[test]
    fn test_budget_warning_threshold() {
        let budget = LatencyBudget::new(12);
        thread::sleep(Duration::from_millis(10));
        assert!(budget.is_warning());
    }

    #[test]
    fn test_tracker_checkpoint_records_phase() {
        let mut tracker = LatencyTracker::new(10_000);
        thread::sleep(Duration::from_millis(10));
        let checkpoint = tracker.checkpoint("compress");
        assert_eq!(checkpoint.phase, "compress");
        assert_eq!(tracker.phases.len(), 1);
        assert!(checkpoint.elapsed_ms >= 10);
    }

    #[test]
    fn test_tracker_multiple_phases() {
        let mut tracker = LatencyTracker::new(10_000);
        for phase in ["compress", "upstream", "shape"] {
            thread::sleep(Duration::from_millis(10));
            tracker.checkpoint(phase);
        }
        assert_eq!(tracker.phases.len(), 3);
    }

    #[test]
    fn test_tracker_report_within_budget() {
        let tracker = LatencyTracker::new(10_000);
        assert!(tracker.report().within_budget);
    }

    #[test]
    fn test_tracker_report_slowest_phase() {
        let mut tracker = LatencyTracker::new(10_000);
        tracker.phases = vec![("compress".to_owned(), 5), ("upstream".to_owned(), 20)];
        assert_eq!(tracker.report().slowest_phase.as_deref(), Some("upstream"));
    }

    #[test]
    fn test_format_report_output() {
        let report = LatencyReport {
            total_elapsed_ms: 234,
            budget_ms: 5000,
            within_budget: true,
            phases: vec![PhaseMetric {
                name: "compress".to_owned(),
                elapsed_ms: 45,
                pct_of_total: 19.2,
            }],
            slowest_phase: Some("compress".to_owned()),
        };
        let output = format_latency_report(&report);
        assert!(output.contains("234ms / 5000ms budget"));
        assert!(output.contains("compress: 45ms (19.2%)"));
    }

    #[test]
    fn test_percentile_calculation() {
        let _guard = SAMPLE_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        clear_samples();
        for sample in 1..=100 {
            record_latency(sample);
        }
        assert_eq!(latency_percentile(50.0), Some(50));
        assert_eq!(latency_percentile(95.0), Some(95));
        assert_eq!(latency_percentile(99.0), Some(99));
        clear_samples();
    }

    #[test]
    fn test_latency_summary_empty() {
        let _guard = SAMPLE_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        clear_samples();
        assert!(latency_summary().is_none());
    }

    #[test]
    fn test_record_latency_bounded() {
        let _guard = SAMPLE_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        clear_samples();
        for sample in 0..5000 {
            record_latency(sample);
        }
        let summary = latency_summary().expect("samples should produce a summary");
        assert_eq!(summary.sample_count, 4096);
        assert_eq!(summary.max_ms, 4999);
        clear_samples();
    }
}
