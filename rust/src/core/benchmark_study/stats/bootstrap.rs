//! Bootstrap confidence intervals.
//!
//! Extracted pattern from `eval_ab::report` — SplitMix64 PRNG, 2000-iter default.

/// SplitMix64 PRNG (no external dependency, deterministic).
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn next_usize(&mut self, max: usize) -> usize {
        (self.next_u64() % max as u64) as usize
    }
}

/// Bootstrap confidence interval for the mean of `values`.
///
/// Returns `(ci_low, ci_high)` at the 95% level (2.5th and 97.5th percentiles).
pub(crate) fn bootstrap_ci(values: &[f64], iterations: usize, seed: u64) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }

    let n = values.len();
    let mut rng = SplitMix64::new(seed);
    let mut means: Vec<f64> = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let mut sum = 0.0;
        for _ in 0..n {
            sum += values[rng.next_usize(n)];
        }
        means.push(sum / n as f64);
    }

    means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let lo = (iterations as f64 * 0.025).floor() as usize;
    let hi = (iterations as f64 * 0.975).ceil() as usize;

    (means[lo.min(iterations - 1)], means[hi.min(iterations - 1)])
}

/// Default bootstrap configuration.
pub(crate) const DEFAULT_ITERATIONS: usize = 2000;
pub(crate) const DEFAULT_SEED: u64 = 0x5EED_5EED_5EED_5EED;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_deterministic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (lo1, hi1) = bootstrap_ci(&values, 1000, DEFAULT_SEED);
        let (lo2, hi2) = bootstrap_ci(&values, 1000, DEFAULT_SEED);
        assert_eq!(lo1, lo2);
        assert_eq!(hi1, hi2);
    }

    #[test]
    fn bootstrap_contains_mean() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (lo, hi) = bootstrap_ci(&values, 2000, DEFAULT_SEED);
        let mean = 3.0;
        assert!(lo <= mean && mean <= hi);
    }

    #[test]
    fn bootstrap_empty_input() {
        let (lo, hi) = bootstrap_ci(&[], 1000, DEFAULT_SEED);
        assert_eq!(lo, 0.0);
        assert_eq!(hi, 0.0);
    }
}
