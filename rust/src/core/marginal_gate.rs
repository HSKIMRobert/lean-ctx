//! Marginal Information Gate (#1308).
//!
//! Checks whether a tool response provides enough new information to justify
//! delivery to the model. Based on COMI's MIG principle (arXiv 2602.01719):
//! content should only be delivered if its information gain relative to what's
//! already in context exceeds a threshold.

use std::collections::HashSet;

/// MIG decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GateDecision {
    /// Content provides enough new information — deliver it.
    Pass,
    /// Content is mostly redundant — suppress and return a stub.
    Suppress {
        novelty_ratio: u8,
        reason: &'static str,
    },
}

/// Configuration for the marginal information gate.
#[derive(Debug, Clone)]
pub(crate) struct GateConfig {
    /// Minimum fraction of novel tokens required to pass. Range 0.0–1.0.
    pub novelty_threshold: f64,
    /// Minimum absolute novel tokens to pass regardless of ratio.
    pub min_novel_tokens: usize,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            novelty_threshold: 0.20,
            min_novel_tokens: 50,
        }
    }
}

/// Check whether `response` provides sufficient new information
/// given `already_delivered` content chunks.
///
/// Uses line-level deduplication: a response line that appears verbatim
/// in any previously delivered chunk is considered redundant.
pub(crate) fn check_information_gain(
    response: &str,
    already_delivered: &[&str],
    config: &GateConfig,
) -> GateDecision {
    if already_delivered.is_empty() || response.is_empty() {
        return GateDecision::Pass;
    }

    let delivered_lines: HashSet<&str> = already_delivered
        .iter()
        .flat_map(|chunk| chunk.lines())
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    let response_lines: Vec<&str> = response
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    if response_lines.is_empty() {
        return GateDecision::Pass;
    }

    let novel_count = response_lines
        .iter()
        .filter(|line| !delivered_lines.contains(*line))
        .count();

    let novelty_ratio = novel_count as f64 / response_lines.len() as f64;

    if novel_count >= config.min_novel_tokens || novelty_ratio >= config.novelty_threshold {
        return GateDecision::Pass;
    }

    let ratio_pct = (novelty_ratio * 100.0) as u8;
    GateDecision::Suppress {
        novelty_ratio: ratio_pct,
        reason: "content mostly redundant with previously delivered context",
    }
}

/// Format a suppression stub when the gate blocks delivery.
pub(crate) fn suppression_stub(path: &str, total_lines: usize, novelty_pct: u8) -> String {
    format!(
        "[MIG: {path} ({total_lines} lines) — {novelty_pct}% novel, below threshold. \
         Use ctx_read with lines= for specific sections.]"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_when_no_prior_delivery() {
        let decision = check_information_gain("fn main() {}", &[], &GateConfig::default());
        assert_eq!(decision, GateDecision::Pass);
    }

    #[test]
    fn pass_when_content_is_novel() {
        let prior = "fn alpha() { 1 }\nfn beta() { 2 }";
        let response = "fn gamma() { 3 }\nfn delta() { 4 }\nfn epsilon() { 5 }";
        let decision = check_information_gain(response, &[prior], &GateConfig::default());
        assert_eq!(decision, GateDecision::Pass);
    }

    #[test]
    fn suppress_when_mostly_redundant() {
        let prior =
            "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10";
        let response =
            "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10";
        let config = GateConfig {
            novelty_threshold: 0.20,
            min_novel_tokens: 50,
        };
        let decision = check_information_gain(response, &[prior], &config);
        match decision {
            GateDecision::Suppress { novelty_ratio, .. } => {
                assert_eq!(novelty_ratio, 0, "0% novel");
            }
            GateDecision::Pass => panic!("should have been suppressed"),
        }
    }

    #[test]
    fn pass_when_above_min_novel_tokens() {
        let prior = "old line";
        let novel_lines: Vec<String> = (0..60).map(|i| format!("new line {i}")).collect();
        let response = novel_lines.join("\n");
        let config = GateConfig {
            novelty_threshold: 0.99,
            min_novel_tokens: 50,
        };
        let decision = check_information_gain(&response, &[prior], &config);
        assert_eq!(decision, GateDecision::Pass);
    }

    #[test]
    fn suppression_stub_format() {
        let stub = suppression_stub("src/db.py", 850, 5);
        assert!(stub.contains("src/db.py"));
        assert!(stub.contains("5% novel"));
        assert!(stub.contains("below threshold"));
    }
}
