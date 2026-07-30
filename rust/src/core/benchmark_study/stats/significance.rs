//! Non-inferiority and significance tests for benchmark comparison.

use super::bootstrap;

/// Non-inferiority test result.
#[derive(Debug, Clone)]
pub(crate) struct NonInferiorityResult {
    pub ci_low: f64,
    pub ci_high: f64,
    pub margin: f64,
    pub is_non_inferior: bool,
    pub is_superior: bool,
}

/// Test whether treatment is non-inferior to baseline within `margin`.
///
/// Uses paired differences: `treatment[i] - baseline[i]`.
/// Non-inferior if `ci_low >= -margin`.
/// Superior if `ci_low > 0`.
pub(crate) fn non_inferiority_test(
    baseline: &[f64],
    treatment: &[f64],
    margin: f64,
) -> NonInferiorityResult {
    assert_eq!(baseline.len(), treatment.len(), "paired data required");

    let diffs: Vec<f64> = baseline
        .iter()
        .zip(treatment.iter())
        .map(|(b, t)| t - b)
        .collect();

    let (ci_low, ci_high) = bootstrap::bootstrap_ci(
        &diffs,
        bootstrap::DEFAULT_ITERATIONS,
        bootstrap::DEFAULT_SEED,
    );

    NonInferiorityResult {
        ci_low,
        ci_high,
        margin,
        is_non_inferior: ci_low >= -margin,
        is_superior: ci_low > 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_distributions_are_non_inferior() {
        let baseline = vec![0.8, 0.9, 0.7, 0.85, 0.95];
        let treatment = baseline.clone();
        let result = non_inferiority_test(&baseline, &treatment, 0.03);
        assert!(result.is_non_inferior);
    }

    #[test]
    fn much_worse_treatment_fails() {
        let baseline = vec![0.9, 0.95, 0.85, 0.9, 0.92];
        let treatment = vec![0.1, 0.2, 0.15, 0.1, 0.12];
        let result = non_inferiority_test(&baseline, &treatment, 0.03);
        assert!(!result.is_non_inferior);
    }
}
