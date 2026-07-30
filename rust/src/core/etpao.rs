//! ETPAO: Effective Tokens per Accepted Outcome (#1318).
//!
//! The canonical efficiency metric that accounts for the full provider
//! cost structure: fresh input, cached input, output, and reasoning tokens
//! have different costs and should be weighted accordingly.

use serde::{Deserialize, Serialize};

/// Provider pricing rates (per million tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TokenPricing {
    pub fresh_input: f64,
    pub cached_input: f64,
    pub output: f64,
    pub reasoning: f64,
}

impl Default for TokenPricing {
    fn default() -> Self {
        Self {
            fresh_input: 3.00,
            cached_input: 0.30,
            output: 15.00,
            reasoning: 15.00,
        }
    }
}

/// Token usage record for a single session/task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct TokenUsage {
    pub fresh_input: u64,
    pub cached_input: u64,
    pub output: u64,
    pub reasoning: u64,
}

impl TokenUsage {
    /// Cost-weighted token count using provider pricing.
    pub(crate) fn weighted_tokens(&self, pricing: &TokenPricing) -> f64 {
        let normalize = pricing.fresh_input;
        if normalize == 0.0 {
            return 0.0;
        }
        self.fresh_input as f64
            + (self.cached_input as f64 * pricing.cached_input / normalize)
            + (self.output as f64 * pricing.output / normalize)
            + (self.reasoning as f64 * pricing.reasoning / normalize)
    }

    /// Total raw token count (unweighted).
    pub(crate) fn total_raw(&self) -> u64 {
        self.fresh_input + self.cached_input + self.output + self.reasoning
    }

    /// Estimated cost in USD.
    pub(crate) fn cost_usd(&self, pricing: &TokenPricing) -> f64 {
        (self.fresh_input as f64 * pricing.fresh_input
            + self.cached_input as f64 * pricing.cached_input
            + self.output as f64 * pricing.output
            + self.reasoning as f64 * pricing.reasoning)
            / 1_000_000.0
    }
}

/// ETPAO comparison between lean-ctx and baseline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EtpaoReport {
    pub leanctx_usage: TokenUsage,
    pub baseline_usage: TokenUsage,
    pub leanctx_weighted: f64,
    pub baseline_weighted: f64,
    pub delta_pct: f64,
    pub leanctx_cost_usd: f64,
    pub baseline_cost_usd: f64,
    pub cost_delta_pct: f64,
}

impl EtpaoReport {
    /// Compute ETPAO comparison.
    pub(crate) fn compute(
        leanctx: TokenUsage,
        baseline: TokenUsage,
        pricing: &TokenPricing,
    ) -> Self {
        let lw = leanctx.weighted_tokens(pricing);
        let bw = baseline.weighted_tokens(pricing);
        let delta_pct = if bw > 0.0 {
            ((lw - bw) / bw) * 100.0
        } else {
            0.0
        };

        let lc = leanctx.cost_usd(pricing);
        let bc = baseline.cost_usd(pricing);
        let cost_delta = if bc > 0.0 {
            ((lc - bc) / bc) * 100.0
        } else {
            0.0
        };

        Self {
            leanctx_usage: leanctx,
            baseline_usage: baseline,
            leanctx_weighted: lw,
            baseline_weighted: bw,
            delta_pct,
            leanctx_cost_usd: lc,
            baseline_cost_usd: bc,
            cost_delta_pct: cost_delta,
        }
    }

    /// True if lean-ctx is cheaper than baseline.
    pub(crate) fn is_cost_efficient(&self) -> bool {
        self.cost_delta_pct < 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_tokens_accounts_for_pricing() {
        let usage = TokenUsage {
            fresh_input: 1_000_000,
            cached_input: 1_000_000,
            output: 100_000,
            reasoning: 0,
        };
        let pricing = TokenPricing::default();
        let weighted = usage.weighted_tokens(&pricing);
        // fresh: 1M * 1.0 + cached: 1M * 0.1 + output: 100k * 5.0
        assert!((weighted - 1_600_000.0).abs() < 1.0);
    }

    #[test]
    fn cost_usd_calculation() {
        let usage = TokenUsage {
            fresh_input: 1_000_000,
            cached_input: 0,
            output: 0,
            reasoning: 0,
        };
        let cost = usage.cost_usd(&TokenPricing::default());
        assert!((cost - 3.00).abs() < 0.01);
    }

    #[test]
    fn etpao_report_negative_delta_means_savings() {
        let leanctx = TokenUsage {
            fresh_input: 500_000,
            cached_input: 500_000,
            output: 50_000,
            reasoning: 0,
        };
        let baseline = TokenUsage {
            fresh_input: 1_000_000,
            cached_input: 0,
            output: 50_000,
            reasoning: 0,
        };
        let report = EtpaoReport::compute(leanctx, baseline, &TokenPricing::default());
        assert!(report.is_cost_efficient());
        assert!(report.delta_pct < 0.0);
    }

    #[test]
    fn etpao_report_positive_delta_means_overhead() {
        let leanctx = TokenUsage {
            fresh_input: 1_200_000,
            cached_input: 0,
            output: 50_000,
            reasoning: 0,
        };
        let baseline = TokenUsage {
            fresh_input: 1_000_000,
            cached_input: 0,
            output: 50_000,
            reasoning: 0,
        };
        let report = EtpaoReport::compute(leanctx, baseline, &TokenPricing::default());
        assert!(!report.is_cost_efficient());
        assert!(report.delta_pct > 0.0);
    }
}
