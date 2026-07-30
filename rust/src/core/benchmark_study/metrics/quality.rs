//! Quality retention metrics.

/// Quality retained: treatment pass rate as percentage of baseline pass rate.
pub(crate) fn quality_retained_pct(baseline_rate: f64, treatment_rate: f64) -> f64 {
    if baseline_rate <= 0.0 {
        return 0.0;
    }
    treatment_rate / baseline_rate * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_retention() {
        assert!((quality_retained_pct(0.95, 0.95) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn partial_retention() {
        let r = quality_retained_pct(0.90, 0.81);
        assert!((r - 90.0).abs() < 1e-9);
    }
}
