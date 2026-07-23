//! Adaptive compression signals exposed through the Context Kernel.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::kernel_config;

static BOUNCE_RATE_BITS: AtomicU64 = AtomicU64::new(0.0_f64.to_bits());
static SIGNALS_RECEIVED: AtomicUsize = AtomicUsize::new(0);

/// Kernel-level recommendation for adjusting compression depth.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub enum KernelCompressionAdvice {
    /// No change to current compression.
    #[default]
    Maintain,
    /// Reduce compression because users are bouncing too often.
    Reduce,
    /// Increase compression because users accept compressed output.
    Increase,
}

/// Snapshot of the kernel's adaptive compression signal state.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct AdaptiveSummary {
    /// Most recently observed bounce rate.
    pub current_bounce_rate: f64,
    /// Compression adjustment advised by the current signal.
    pub advice: KernelCompressionAdvice,
    /// Number of bounce-rate signals recorded since the last reset.
    pub signals_received: usize,
}

/// Advises how compression should change for a measured bounce rate.
#[must_use]
pub fn compression_advice(bounce_rate: f64) -> KernelCompressionAdvice {
    if !kernel_config::is_enabled() {
        return KernelCompressionAdvice::Maintain;
    }
    if bounce_rate > 0.3 {
        KernelCompressionAdvice::Reduce
    } else if bounce_rate < 0.05 {
        KernelCompressionAdvice::Increase
    } else {
        KernelCompressionAdvice::Maintain
    }
}

/// Stores the latest bounce-rate signal for kernel consumers.
pub fn update_bounce_signal(bounce_rate: f64) {
    BOUNCE_RATE_BITS.store(bounce_rate.to_bits(), Ordering::Relaxed);
    SIGNALS_RECEIVED.fetch_add(1, Ordering::Relaxed);
}

/// Returns the most recently stored bounce rate.
#[must_use]
pub fn current_bounce_rate() -> f64 {
    f64::from_bits(BOUNCE_RATE_BITS.load(Ordering::Relaxed))
}

/// Returns the current adaptive compression state.
#[must_use]
pub fn adaptive_summary() -> AdaptiveSummary {
    let current_bounce_rate = current_bounce_rate();
    AdaptiveSummary {
        current_bounce_rate,
        advice: compression_advice(current_bounce_rate),
        signals_received: SIGNALS_RECEIVED.load(Ordering::Relaxed),
    }
}

/// Clears all adaptive compression signal state.
pub fn reset() {
    BOUNCE_RATE_BITS.store(0.0_f64.to_bits(), Ordering::Relaxed);
    SIGNALS_RECEIVED.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::{
        KernelCompressionAdvice, adaptive_summary, compression_advice, reset, update_bounce_signal,
    };
    use crate::core::context_kernel::kernel_config::{self, KernelFeatures};

    fn setup() -> std::sync::MutexGuard<'static, ()> {
        let guard = kernel_config::KERNEL_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        kernel_config::reset_features();
        reset();
        guard
    }

    #[test]
    fn high_bounce_advises_reduce() {
        let _guard = setup();
        assert_eq!(compression_advice(0.5), KernelCompressionAdvice::Reduce);
    }

    #[test]
    fn low_bounce_advises_increase() {
        let _guard = setup();
        assert_eq!(compression_advice(0.01), KernelCompressionAdvice::Increase);
    }

    #[test]
    fn moderate_bounce_maintains() {
        let _guard = setup();
        assert_eq!(compression_advice(0.15), KernelCompressionAdvice::Maintain);
    }

    #[test]
    fn disabled_kernel_always_maintains() {
        let _guard = setup();
        let features = KernelFeatures {
            enabled: false,
            ..KernelFeatures::default()
        };
        kernel_config::update_features(features);
        assert_eq!(compression_advice(0.9), KernelCompressionAdvice::Maintain);
    }

    #[test]
    fn summary_reports_latest_signal_and_count() {
        let _guard = setup();
        update_bounce_signal(0.4);
        update_bounce_signal(0.2);
        let summary = adaptive_summary();
        assert_eq!(summary.current_bounce_rate, 0.2);
        assert_eq!(summary.advice, KernelCompressionAdvice::Maintain);
        assert_eq!(summary.signals_received, 2);
    }
}
