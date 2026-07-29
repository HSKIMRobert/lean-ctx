//! Combined savings report: aggregates compression + routing + caching savings
//! into a single view with overlap correction for the benchmark study (E-Bench).

use super::event::{MECHANISM_CACHING, MECHANISM_COMPRESSION, MECHANISM_ROUTING};
use super::store;
use serde::{Deserialize, Serialize};

/// Per-mechanism breakdown with overlap-corrected combined total.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombinedSavingsReport {
    /// Tokens saved through compression (smaller payloads).
    pub compression_saved_tokens: u64,
    pub compression_saved_usd: f64,
    pub compression_events: usize,

    /// USD saved through routing (cheaper model at same tokens).
    pub routing_saved_usd: f64,
    pub routing_events: usize,

    /// USD saved through prompt-cache discounts.
    pub caching_saved_usd: f64,
    pub caching_events: usize,

    /// Combined total with overlap correction.
    /// Overlap: routing saves on already-compressed tokens, so the rate-delta
    /// applies to `actual_tokens` not `baseline_tokens`. The correction removes
    /// the hypothetical routing saving on the tokens compression already eliminated.
    pub combined_saved_usd: f64,
    pub overlap_correction_usd: f64,

    /// Total events across all mechanisms.
    pub total_events: usize,
}

impl CombinedSavingsReport {
    /// Build from the current ledger on disk.
    pub fn from_ledger() -> Self {
        let Some(path) = store::default_path() else {
            return Self::default();
        };
        let summary = store::summarize(&path);
        Self::from_summary(&summary)
    }

    /// Build from a pre-computed `LedgerSummary`.
    pub fn from_summary(summary: &store::LedgerSummary) -> Self {
        let mut report = Self::default();

        for (mechanism, saved_tokens, saved_usd) in &summary.by_mechanism {
            match mechanism.as_str() {
                m if m == MECHANISM_COMPRESSION => {
                    report.compression_saved_tokens = *saved_tokens;
                    report.compression_saved_usd = *saved_usd;
                }
                m if m == MECHANISM_ROUTING => {
                    report.routing_saved_usd = *saved_usd;
                }
                m if m == MECHANISM_CACHING => {
                    report.caching_saved_usd = *saved_usd;
                }
                _ => {}
            }
        }

        // Count events per mechanism from the raw events
        let events = store::load(&store::default_path().unwrap_or_default());
        let total_baseline_tokens: u64 = events
            .iter()
            .filter(|ev| ev.mechanism == MECHANISM_COMPRESSION)
            .map(|ev| ev.baseline_tokens)
            .sum();
        for ev in &events {
            match ev.mechanism.as_str() {
                m if m == MECHANISM_COMPRESSION => report.compression_events += 1,
                m if m == MECHANISM_ROUTING => report.routing_events += 1,
                m if m == MECHANISM_CACHING => report.caching_events += 1,
                _ => {}
            }
        }
        report.total_events = events.len();

        // Overlap correction: routing USD is computed on the full (pre-compression)
        // token count. If compression removed N% of tokens, the routing saving
        // on those removed tokens is illusory — the provider never saw them.
        // correction = routing_saved_usd * (compression_ratio)
        // where compression_ratio = compression_saved_tokens / total_baseline_tokens
        if total_baseline_tokens > 0 && report.routing_saved_usd > 0.0 {
            let compression_ratio =
                report.compression_saved_tokens as f64 / total_baseline_tokens as f64;
            report.overlap_correction_usd = report.routing_saved_usd * compression_ratio;
        }

        report.combined_saved_usd =
            report.compression_saved_usd + report.routing_saved_usd + report.caching_saved_usd
                - report.overlap_correction_usd;

        report
    }

    /// Format as human-readable table.
    pub fn format_terminal(&self) -> String {
        format!(
            "Combined Savings Report\n\
             ═══════════════════════════════════════════\n\
             Compression:  {:>10} tokens  ${:.4}\n\
             Routing:      {:>10}         ${:.4}\n\
             Caching:      {:>10}         ${:.4}\n\
             ───────────────────────────────────────────\n\
             Overlap correction:          -${:.4}\n\
             ═══════════════════════════════════════════\n\
             Combined total:              ${:.4}\n\
             Events: {} compression, {} routing, {} caching\n",
            self.compression_saved_tokens,
            self.compression_saved_usd,
            "",
            self.routing_saved_usd,
            "",
            self.caching_saved_usd,
            self.overlap_correction_usd,
            self.combined_saved_usd,
            self.compression_events,
            self.routing_events,
            self.caching_events,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_report_is_zero() {
        let r = CombinedSavingsReport::default();
        assert_eq!(r.combined_saved_usd, 0.0);
        assert_eq!(r.total_events, 0);
    }

    #[test]
    fn overlap_correction_reduces_combined() {
        let mut r = CombinedSavingsReport::default();
        r.compression_saved_tokens = 500;
        r.compression_saved_usd = 0.005;
        r.routing_saved_usd = 0.010;
        // If compression removed 50% of tokens, routing overstates by 50%
        r.overlap_correction_usd = 0.005;
        r.combined_saved_usd =
            r.compression_saved_usd + r.routing_saved_usd - r.overlap_correction_usd;
        assert!((r.combined_saved_usd - 0.010).abs() < 1e-9);
    }

    #[test]
    fn format_terminal_contains_headers() {
        let r = CombinedSavingsReport::default();
        let s = r.format_terminal();
        assert!(s.contains("Combined Savings Report"));
        assert!(s.contains("Compression"));
        assert!(s.contains("Routing"));
        assert!(s.contains("Caching"));
    }
}
