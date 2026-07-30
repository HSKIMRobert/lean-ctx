//! Pass@k metric computation.

/// Compute pass@1 from a list of boolean outcomes.
pub(crate) fn pass_at_1(outcomes: &[bool]) -> f64 {
    if outcomes.is_empty() {
        return 0.0;
    }
    let passed = outcomes.iter().filter(|&&p| p).count();
    passed as f64 / outcomes.len() as f64
}

/// Compute pass@k using the unbiased estimator from the Codex paper.
/// `n` = total samples, `c` = correct samples, `k` = k value.
pub(crate) fn pass_at_k(n: usize, c: usize, k: usize) -> f64 {
    if n < k {
        return 0.0;
    }
    if c == 0 {
        return 0.0;
    }
    1.0 - comb_ratio(n - c, k, n)
}

/// Compute C(n-c, k) / C(n, k) using the product formula to avoid overflow.
fn comb_ratio(n_minus_c: usize, k: usize, n: usize) -> f64 {
    let mut ratio = 1.0;
    for i in 0..k {
        if n_minus_c < i {
            return 0.0;
        }
        ratio *= (n_minus_c - i) as f64 / (n - i) as f64;
    }
    ratio
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_at_1_all_correct() {
        assert!((pass_at_1(&[true, true, true]) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn pass_at_1_none_correct() {
        assert!((pass_at_1(&[false, false]) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn pass_at_1_mixed() {
        assert!((pass_at_1(&[true, false, true, false]) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn pass_at_1_empty() {
        assert_eq!(pass_at_1(&[]), 0.0);
    }

    #[test]
    fn pass_at_k_basic() {
        // 10 samples, 5 correct, k=1 → 1 - C(5,1)/C(10,1) = 1 - 5/10 = 0.5
        assert!((pass_at_k(10, 5, 1) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn pass_at_k_all_correct() {
        assert!((pass_at_k(10, 10, 1) - 1.0).abs() < 1e-9);
    }
}
