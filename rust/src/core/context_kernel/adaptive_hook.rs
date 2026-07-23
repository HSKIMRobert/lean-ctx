//! Adapts kernel compression advice from observed context-read bounces.

use super::adaptive_bridge::KernelCompressionAdvice;

/// Compression guidance derived from bounce-tracker observations.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct CompressionAdvice {
    /// Whether compression should be reduced.
    pub should_reduce: bool,
    /// Bounce rate used to produce the advice.
    pub bounce_rate: f64,
    /// Kernel recommendation for the observed bounce rate.
    pub advice: KernelCompressionAdvice,
}

impl CompressionAdvice {
    fn maintain() -> Self {
        Self {
            should_reduce: false,
            bounce_rate: 0.0,
            advice: KernelCompressionAdvice::Maintain,
        }
    }

    fn from_rate(bounce_rate: f64) -> Self {
        let advice = super::adaptive_bridge::compression_advice(bounce_rate);
        Self {
            should_reduce: advice == KernelCompressionAdvice::Reduce,
            bounce_rate,
            advice,
        }
    }
}

/// Updates the adaptive bridge from the process-wide bounce tracker.
pub fn update_from_bounce_tracker() {
    if !super::kernel_config::is_enabled() {
        return;
    }
    let tracker = crate::core::bounce_tracker::global()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let total_bounces = tracker.total_bounces();
    let _total_wasted_tokens = tracker.total_wasted_tokens();
    let denominator = total_bounces.saturating_add(100).max(1);
    let rate = total_bounces as f64 / denominator as f64;
    drop(tracker);
    super::adaptive_bridge::update_bounce_signal(rate);
}

/// Returns adaptive compression guidance for a path's extension.
#[must_use]
pub fn advice_for_path(path: &str) -> CompressionAdvice {
    if !super::kernel_config::is_enabled() {
        return CompressionAdvice::maintain();
    }
    let tracker = crate::core::bounce_tracker::global()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    tracker
        .bounce_rate_for_extension(path)
        .map_or_else(CompressionAdvice::maintain, CompressionAdvice::from_rate)
}

/// Returns adaptive compression guidance for the latest global bounce signal.
#[must_use]
pub fn global_advice() -> CompressionAdvice {
    CompressionAdvice::from_rate(super::adaptive_bridge::current_bounce_rate())
}

/// Clears adaptive compression signal state.
pub fn reset() {
    super::adaptive_bridge::reset();
}

#[cfg(test)]
mod tests {
    use super::{advice_for_path, update_from_bounce_tracker};
    use crate::core::context_kernel::{adaptive_bridge, kernel_config};

    fn setup() -> std::sync::MutexGuard<'static, ()> {
        let guard = kernel_config::KERNEL_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        kernel_config::reset_features();
        adaptive_bridge::reset();
        let mut tracker = crate::core::bounce_tracker::global()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *tracker = crate::core::bounce_tracker::BounceTracker::new();
        drop(tracker);
        guard
    }

    #[test]
    fn update_feeds_bridge() {
        let _guard = setup();
        crate::core::bounce_tracker::global()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .record_expansion(None, 20);
        update_from_bounce_tracker();
        assert!(adaptive_bridge::current_bounce_rate() > 0.0);
    }

    #[test]
    fn advice_for_unknown_path() {
        let _guard = setup();
        assert_eq!(
            advice_for_path("unknown.rs").advice,
            adaptive_bridge::KernelCompressionAdvice::Maintain
        );
    }

    #[test]
    fn disabled_kernel_noop() {
        let _guard = setup();
        let mut features = kernel_config::features();
        features.enabled = false;
        kernel_config::update_features(features);
        update_from_bounce_tracker();
        assert_eq!(
            advice_for_path("disabled.rs").advice,
            adaptive_bridge::KernelCompressionAdvice::Maintain
        );
    }
}
