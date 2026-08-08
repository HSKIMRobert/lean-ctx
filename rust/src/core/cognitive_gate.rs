//! Central feature gate for science-driven context intelligence.
//!
//! Reads `cognitive_mode` from config once per call and returns whether
//! a feature category is enabled. Two tiers:
//! - **Basic** (IB intent, semantic chunking): enabled in `Basic` and `Full` modes
//! - **Full** (FSRS, Wasserstein, graph, verbosity, prefetch, stigmergy): only in `Full` mode

use crate::core::config::{CognitiveMode, Config};

/// Returns true if basic science features (intent classification, semantic chunking) are active.
pub(crate) fn basic_science_enabled() -> bool {
    !matches!(Config::load().cognitive_mode, CognitiveMode::Off)
}

/// Returns true if full science features (FSRS, OT allocation, graph expansion, etc.) are active.
pub(crate) fn full_science_enabled() -> bool {
    matches!(Config::load().cognitive_mode, CognitiveMode::Full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_science_enabled_when_not_off() {
        // Default config is Basic — basic tier must be on, full tier off.
        assert!(basic_science_enabled());
        assert!(!full_science_enabled());
    }
}
