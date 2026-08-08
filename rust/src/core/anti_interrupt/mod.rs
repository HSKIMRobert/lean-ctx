//! Anti-Interruption Score (F7) — cognitive impact metrics beyond token savings.
//!
//! Tracks interruption events (echo repetitions, redundant reads, context switches)
//! and computes a session-level score measuring cognitive interruptions prevented.

mod metrics;
mod tracker;

pub(crate) use metrics::compute_impact;
pub use tracker::InterruptionEvent;
#[cfg(test)]
pub(crate) use tracker::{TEST_LOCK, reset_session};
pub(crate) use tracker::{record_interruption, session_interruptions};

/// Record a prevented redundant read on a background thread (F7).
pub(crate) fn spawn_redundant_read(path: impl Into<String>) {
    if crate::core::cognitive_gate::full_science_enabled() {
        let path = path.into();
        std::thread::spawn(move || {
            record_interruption(InterruptionEvent::RedundantRead { path }, true);
        });
    }
}

/// Record bounce-waste tokens on a background thread (F7).
pub(crate) fn spawn_bounce_waste(tokens: u64) {
    if crate::core::cognitive_gate::full_science_enabled() {
        std::thread::spawn(move || {
            record_interruption(InterruptionEvent::BounceWaste { tokens }, false);
        });
    }
}
