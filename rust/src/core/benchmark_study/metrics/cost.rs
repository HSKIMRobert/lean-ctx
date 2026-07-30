//! Cost metrics for benchmark comparison.

/// Cost per 1000 prompts.
pub(crate) fn cost_per_1k(total_cost_usd: f64, total_tasks: usize) -> f64 {
    if total_tasks == 0 {
        return 0.0;
    }
    total_cost_usd / total_tasks as f64 * 1000.0
}

/// Cost savings as a percentage.
pub(crate) fn savings_pct(baseline_cost: f64, treatment_cost: f64) -> f64 {
    if baseline_cost <= 0.0 {
        return 0.0;
    }
    (1.0 - treatment_cost / baseline_cost) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn savings_50_pct() {
        assert!((savings_pct(100.0, 50.0) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn savings_zero_baseline() {
        assert_eq!(savings_pct(0.0, 50.0), 0.0);
    }
}
