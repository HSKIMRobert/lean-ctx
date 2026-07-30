//! Session-level cumulative context budget (#1307).
//!
//! Tracks total fresh tokens delivered across the session and applies
//! progressive compression tiers as usage grows:
//!
//! | Tier    | Cumulative usage  | Default strategy                    |
//! |---------|-------------------|-------------------------------------|
//! | Green   | 0–50%             | No intervention                     |
//! | Yellow  | 50–75%            | Prefer compressed modes (signatures)|
//! | Orange  | 75–90%            | Force map/signatures on all reads   |
//! | Red     | 90–100%           | Reference-only, expand on demand    |

use std::sync::atomic::{AtomicUsize, Ordering};

static CUMULATIVE_TOKENS: AtomicUsize = AtomicUsize::new(0);

/// Session budget pressure tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PressureTier {
    Green,
    Yellow,
    Orange,
    Red,
}

impl PressureTier {
    /// Suggested read mode downgrade for this pressure tier.
    pub(crate) fn suggested_mode(&self) -> Option<&'static str> {
        match self {
            PressureTier::Green => None,
            PressureTier::Yellow => Some("signatures"),
            PressureTier::Orange => Some("map"),
            PressureTier::Red => Some("reference"),
        }
    }

    pub(crate) fn label(&self) -> &'static str {
        match self {
            PressureTier::Green => "green",
            PressureTier::Yellow => "yellow",
            PressureTier::Orange => "orange",
            PressureTier::Red => "red",
        }
    }
}

/// Record `n` fresh tokens delivered in this session.
pub(crate) fn record_delivery(n: usize) {
    CUMULATIVE_TOKENS.fetch_add(n, Ordering::Relaxed);
}

/// Current cumulative fresh token count.
pub(crate) fn cumulative_tokens() -> usize {
    CUMULATIVE_TOKENS.load(Ordering::Relaxed)
}

/// Current pressure tier based on cumulative usage vs session limit.
pub(crate) fn current_tier(session_limit: usize) -> PressureTier {
    if session_limit == 0 {
        return PressureTier::Green;
    }
    let used = cumulative_tokens();
    let ratio = used as f64 / session_limit as f64;
    if ratio >= 0.90 {
        PressureTier::Red
    } else if ratio >= 0.75 {
        PressureTier::Orange
    } else if ratio >= 0.50 {
        PressureTier::Yellow
    } else {
        PressureTier::Green
    }
}

/// Reset session budget (for testing or session restart).
pub(crate) fn reset() {
    CUMULATIVE_TOKENS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_green_when_below_half() {
        assert_eq!(current_tier(1000), PressureTier::Green);
    }

    #[test]
    fn tier_transitions() {
        reset();
        assert_eq!(current_tier(1000), PressureTier::Green);

        record_delivery(500);
        assert_eq!(current_tier(1000), PressureTier::Yellow);

        record_delivery(250);
        assert_eq!(current_tier(1000), PressureTier::Orange);

        record_delivery(150);
        assert_eq!(current_tier(1000), PressureTier::Red);

        reset();
        assert_eq!(current_tier(1000), PressureTier::Green);
    }

    #[test]
    fn tier_green_when_unlimited() {
        record_delivery(999_999);
        assert_eq!(current_tier(0), PressureTier::Green);
        reset();
    }

    #[test]
    fn suggested_modes() {
        assert_eq!(PressureTier::Green.suggested_mode(), None);
        assert_eq!(PressureTier::Yellow.suggested_mode(), Some("signatures"));
        assert_eq!(PressureTier::Orange.suggested_mode(), Some("map"));
        assert_eq!(PressureTier::Red.suggested_mode(), Some("reference"));
    }
}
