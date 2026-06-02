//! Electricity-footprint estimate for saved tokens.
//!
//! This mirrors the website `/metrics` methodology (same `J_PER_TOKEN`) so a user's local
//! "energy saved" figure and the community scoreboard always reconcile.
//!
//! ~0.4 J per saved token is a deliberately conservative midpoint of measured modern
//! inference (Llama-3-70B FP8 on H100 + vLLM ≈ 0.39 J/token, John Snow Labs "Tokens per
//! Joule", 2025; query-level estimates such as Epoch AI's ~0.3 Wh per GPT-4o query imply
//! more). lean-ctx mostly removes cheaper prefill/context tokens, so we never overstate.
//! Real figures vary by model and hardware — this is always surfaced as an estimate.

/// Joules of inference compute avoided per token that lean-ctx kept out of the context.
pub const J_PER_TOKEN: f64 = 0.4;

/// Watt-hours in a full smartphone charge — the relatable yardstick used in the UI.
pub const WH_PER_PHONE_CHARGE: f64 = 12.0;

/// Energy (Wh) saved for a given number of saved tokens. `Wh = tokens · J/token / 3600`.
#[must_use]
pub fn wh_for_tokens(tokens_saved: u64) -> f64 {
    tokens_saved as f64 * J_PER_TOKEN / 3600.0
}

/// Equivalent number of full smartphone charges for a given saved-token count.
#[must_use]
pub fn phone_charges(tokens_saved: u64) -> f64 {
    wh_for_tokens(tokens_saved) / WH_PER_PHONE_CHARGE
}

/// Human-readable energy with adaptive units (`Wh` / `kWh` / `MWh`).
#[must_use]
pub fn format_wh(wh: f64) -> String {
    if !wh.is_finite() || wh <= 0.0 {
        "0 Wh".to_string()
    } else if wh >= 1_000_000.0 {
        format!("{:.1} MWh", wh / 1_000_000.0)
    } else if wh >= 1_000.0 {
        format!("{:.1} kWh", wh / 1_000.0)
    } else {
        format!("{wh:.0} Wh")
    }
}

/// Convenience: formatted energy string straight from a saved-token count.
#[must_use]
pub fn format_for_tokens(tokens_saved: u64) -> String {
    format_wh(wh_for_tokens(tokens_saved))
}

/// Rounded phone-charge equivalent as a display string (e.g. `"≈ 117 phone charges"`),
/// or `None` when the saving is too small to round to at least one charge.
#[must_use]
pub fn phone_charges_hint(tokens_saved: u64) -> Option<String> {
    let charges = phone_charges(tokens_saved);
    if charges < 0.5 {
        return None;
    }
    Some(format!("≈ {} phone charges", charges.round() as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wh_scales_linearly_with_tokens() {
        // 9000 tokens · 0.4 J / 3600 = exactly 1 Wh.
        assert!((wh_for_tokens(9_000) - 1.0).abs() < 1e-9);
        assert!((wh_for_tokens(0)).abs() < 1e-9);
    }

    #[test]
    fn format_picks_adaptive_units() {
        assert_eq!(format_wh(0.0), "0 Wh");
        assert_eq!(format_wh(-5.0), "0 Wh");
        assert_eq!(format_wh(42.4), "42 Wh");
        assert_eq!(format_wh(1_500.0), "1.5 kWh");
        assert_eq!(format_wh(2_500_000.0), "2.5 MWh");
    }

    #[test]
    fn format_for_tokens_matches_methodology() {
        // 12.8M tokens ≈ 1422 Wh ≈ 1.4 kWh (the figure a real user would see).
        assert_eq!(format_for_tokens(12_800_000), "1.4 kWh");
    }

    #[test]
    fn phone_charge_hint_suppressed_when_tiny() {
        assert!(phone_charges_hint(0).is_none());
        // 12 Wh = 1 charge needs 108k tokens; far below that → no hint.
        assert!(phone_charges_hint(1_000).is_none());
        assert!(phone_charges_hint(12_800_000).is_some());
    }
}
