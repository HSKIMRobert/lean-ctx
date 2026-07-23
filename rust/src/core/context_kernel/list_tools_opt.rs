//! Hot-path bridge for MCP list-tools schema optimization.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::{kernel_config, schema_wiring};

static OPTIMIZATIONS_APPLIED: AtomicUsize = AtomicUsize::new(0);
static TOTAL_TOKENS_BEFORE: AtomicUsize = AtomicUsize::new(0);
static TOTAL_TOKENS_SAVED: AtomicUsize = AtomicUsize::new(0);

/// Cumulative statistics for list-tools schema optimization.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SchemaOptSummary {
    /// Number of tool lists optimized.
    pub optimizations_applied: usize,
    /// Total estimated schema tokens removed.
    pub total_tokens_saved: usize,
    /// Percentage of input schema tokens removed.
    pub avg_reduction_percent: f64,
}

/// Returns whether list-tools schemas may be optimized for this client.
#[must_use]
pub fn should_optimize_for_client(client_name: &str) -> bool {
    kernel_config::is_feature_enabled("schema_optimization")
        && schema_wiring::should_optimize(client_name)
}

/// Optimizes tool descriptions for an addressable client when the kernel permits it.
#[must_use]
pub fn optimize_descriptions(
    tools: Vec<(String, String, usize)>,
    client_name: &str,
) -> Vec<(String, String, usize)> {
    if !should_optimize_for_client(client_name) {
        return tools;
    }

    let optimized = schema_wiring::optimize_tool_list(&tools, client_name);
    let tokens_saved = optimized
        .tokens_before
        .saturating_sub(optimized.tokens_after);
    OPTIMIZATIONS_APPLIED.fetch_add(1, Ordering::Relaxed);
    TOTAL_TOKENS_BEFORE.fetch_add(optimized.tokens_before, Ordering::Relaxed);
    TOTAL_TOKENS_SAVED.fetch_add(tokens_saved, Ordering::Relaxed);
    optimized.tools
}

/// Returns cumulative list-tools optimization statistics.
#[must_use]
pub fn schema_opt_summary() -> SchemaOptSummary {
    let optimizations_applied = OPTIMIZATIONS_APPLIED.load(Ordering::Relaxed);
    let total_tokens_before = TOTAL_TOKENS_BEFORE.load(Ordering::Relaxed);
    let total_tokens_saved = TOTAL_TOKENS_SAVED.load(Ordering::Relaxed);
    let avg_reduction_percent = if total_tokens_before == 0 {
        0.0
    } else {
        total_tokens_saved as f64 * 100.0 / total_tokens_before as f64
    };

    SchemaOptSummary {
        optimizations_applied,
        total_tokens_saved,
        avg_reduction_percent,
    }
}

/// Clears cumulative bridge statistics.
pub fn reset() {
    OPTIMIZATIONS_APPLIED.store(0, Ordering::Relaxed);
    TOTAL_TOKENS_BEFORE.store(0, Ordering::Relaxed);
    TOTAL_TOKENS_SAVED.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use std::sync::MutexGuard;

    use super::{optimize_descriptions, reset, schema_opt_summary};
    use crate::core::context_kernel::kernel_config::{self, KERNEL_TEST_LOCK, KernelFeatures};

    fn setup() -> MutexGuard<'static, ()> {
        let guard = KERNEL_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        kernel_config::reset_features();
        reset();
        guard
    }

    fn tools() -> Vec<(String, String, usize)> {
        (0..15)
            .map(|index| (format!("tool_{index}"), "long description ".repeat(250), 3))
            .collect()
    }

    #[test]
    fn disabled_returns_unchanged() {
        let _guard = setup();
        let original = tools();
        let features = KernelFeatures {
            enabled: false,
            ..KernelFeatures::default()
        };
        kernel_config::update_features(features);

        assert_eq!(optimize_descriptions(original.clone(), "cursor"), original);
    }

    #[test]
    fn optimization_shortens_descriptions() {
        let _guard = setup();
        let original = tools();
        let before: usize = original.iter().map(|tool| tool.1.len()).sum();
        let optimized = optimize_descriptions(original, "cursor");
        let after: usize = optimized.iter().map(|tool| tool.1.len()).sum();

        assert!(after < before);
    }

    #[test]
    fn unmanaged_client_unchanged() {
        let _guard = setup();
        let original = tools();

        assert_eq!(
            optimize_descriptions(original.clone(), "unknown-client"),
            original
        );
    }

    #[test]
    fn cursor_client_optimizes() {
        let _guard = setup();

        let _ = optimize_descriptions(tools(), "cursor");

        assert_eq!(schema_opt_summary().optimizations_applied, 1);
    }

    #[test]
    fn summary_tracks_savings() {
        let _guard = setup();

        let _ = optimize_descriptions(tools(), "cursor");
        let summary = schema_opt_summary();

        assert!(summary.total_tokens_saved > 0);
        assert!(summary.avg_reduction_percent > 0.0);
    }

    #[test]
    fn disabled_feature_returns_unchanged() {
        let _guard = setup();
        let original = tools();
        let features = KernelFeatures {
            schema_optimization: false,
            ..KernelFeatures::default()
        };
        kernel_config::update_features(features);

        assert_eq!(optimize_descriptions(original.clone(), "cursor"), original);
    }
}
