//! Net-cost policy for cache-busting rewrites (#986, cache-economics).
//!
//! The cold-prefix repack (#480) re-seeds a leaner prompt cache when the proxy
//! predicts the client-cached prefix has already gone cold (idle past the TTL).
//! Because the entry is *already* expired, the provider re-writes the prefix on
//! the next turn no matter what — so compressing that unavoidable re-write is
//! free savings, **except** for prefixes too small to be cached at all. Repacking
//! one of those only churns the conversation's cache identity for no benefit.
//!
//! This module is the pricing brain that decides when a repack pays:
//!
//! - [`worth_repacking`] — the live gate applied in `anthropic.rs`. It runs
//!   *before* compression, so it can only weigh the measurable precondition: is
//!   the cacheable prefix even large enough to be worth re-seeding?
//! - [`net_cost_decision`] / [`repack_saving_usd`] — the fully priced primitive
//!   (before/after token counts × [`ModelCost`]) for callers that already know
//!   the compressed size (tests today, cache-edit batching later).
//!
//! Pure functions, no globals; gated behind the opt-in `proxy.cache_policy` at
//! the call site so a default proxy keeps today's behaviour exactly.

use serde_json::Value;

use crate::core::gain::model_pricing::ModelCost;
use crate::core::tokens::count_tokens;

/// Minimum cacheable prefix size, in tokens. Anthropic will not cache a prefix
/// below this (1024 for most models; Haiku needs more), so re-seeding a smaller
/// one can never produce a cache the provider would keep — the conservative
/// floor below which a repack is pure churn. Chosen as the documented Anthropic
/// minimum rather than an estimate.
pub const MIN_CACHEABLE_TOKENS: u64 = 1024;

/// Token count of the client-cached prefix `messages[0..cached]`. Measured (not
/// estimated) via the same BPE counter the rest of the proxy uses, so the gate
/// reflects the real prefix the provider would cache.
#[must_use]
pub fn prefix_tokens(messages: &[Value], cached: usize) -> u64 {
    let end = cached.min(messages.len());
    if end == 0 {
        return 0;
    }
    let Ok(serialized) = serde_json::to_string(&messages[..end]) else {
        return 0;
    };
    count_tokens(&serialized) as u64
}

/// Live repack gate (pre-compression). A cold-prefix repack only pays when the
/// cacheable prefix is large enough that the provider will actually cache the
/// re-seeded version; below [`MIN_CACHEABLE_TOKENS`] the repack just churns the
/// conversation's cache key. Applied as an extra AND-condition on the existing
/// repack decision, so enabling the policy can only make repacking *more*
/// conservative — never trigger a rewrite that would not have happened.
#[must_use]
pub fn worth_repacking(messages: &[Value], cached: usize) -> bool {
    prefix_tokens(messages, cached) >= MIN_CACHEABLE_TOKENS
}

/// Cache-write cost saved by re-seeding a compressed prefix instead of the full
/// one, in USD. On a cold prefix the provider re-writes regardless, so the saving
/// is the avoided write of the dropped tokens: `(before − after) × cache_write`.
/// Clamped to `0.0` when compression did not shrink the prefix.
#[must_use]
pub fn repack_saving_usd(before_tokens: u64, after_tokens: u64, cost: &ModelCost) -> f64 {
    let saved = before_tokens.saturating_sub(after_tokens);
    saved as f64 / 1_000_000.0 * cost.cache_write_per_m
}

/// Fully priced repack decision for callers that already know the compressed
/// size. True when the prefix is cacheable *and* re-seeding it strictly lowers
/// the unavoidable cold re-write cost. The precondition mirrors
/// [`worth_repacking`] so the live gate and the priced primitive never disagree.
#[must_use]
pub fn net_cost_decision(before_tokens: u64, after_tokens: u64, cost: &ModelCost) -> bool {
    before_tokens >= MIN_CACHEABLE_TOKENS
        && after_tokens < before_tokens
        && repack_saving_usd(before_tokens, after_tokens, cost) > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn opus() -> ModelCost {
        ModelCost {
            input_per_m: 15.00,
            output_per_m: 75.00,
            cache_write_per_m: 18.75,
            cache_read_per_m: 1.50,
        }
    }

    #[test]
    fn prefix_tokens_zero_when_nothing_cached() {
        let msgs = vec![json!({"role": "user", "content": "hello"})];
        assert_eq!(prefix_tokens(&msgs, 0), 0);
    }

    #[test]
    fn prefix_tokens_counts_only_the_cached_span() {
        let big = "lorem ipsum dolor sit amet ".repeat(50);
        let msgs = vec![
            json!({"role": "user", "content": big}),
            json!({"role": "assistant", "content": "tail not counted"}),
        ];
        let one = prefix_tokens(&msgs, 1);
        let two = prefix_tokens(&msgs, 2);
        assert!(one > 0);
        assert!(two > one, "wider cached span counts more tokens");
    }

    #[test]
    fn worth_repacking_rejects_small_prefix() {
        let msgs = vec![json!({"role": "user", "content": "tiny"})];
        assert!(!worth_repacking(&msgs, 1));
    }

    #[test]
    fn worth_repacking_accepts_large_prefix() {
        // Comfortably above the 1024-token cacheable floor.
        let big = "context engineering ".repeat(1500);
        let msgs = vec![json!({"role": "user", "content": big})];
        assert!(worth_repacking(&msgs, 1));
    }

    #[test]
    fn net_cost_decision_rejects_subcacheable_even_if_smaller() {
        // Below the cacheable floor: a smaller "after" still doesn't pay.
        assert!(!net_cost_decision(500, 200, &opus()));
    }

    #[test]
    fn net_cost_decision_rejects_when_no_shrink() {
        assert!(!net_cost_decision(4000, 4000, &opus()));
        assert!(!net_cost_decision(4000, 5000, &opus()));
    }

    #[test]
    fn net_cost_decision_accepts_real_saving() {
        assert!(net_cost_decision(4000, 2500, &opus()));
        // Saving is the avoided write of the 1500 dropped tokens.
        let saved = repack_saving_usd(4000, 2500, &opus());
        assert!((saved - (1500.0 / 1_000_000.0 * 18.75)).abs() < 1e-9);
    }

    #[test]
    fn repack_saving_is_zero_when_inflated() {
        assert_eq!(repack_saving_usd(1000, 2000, &opus()), 0.0);
    }
}
