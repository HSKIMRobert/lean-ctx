//! Anti-Interruption Score (F7) — cognitive impact metrics beyond token savings.
//!
//! Tracks interruption events (echo repetitions, redundant reads, context switches)
//! and computes a session-level score measuring cognitive interruptions prevented.

mod metrics;
mod tracker;

pub(crate) use metrics::compute_impact;
pub use tracker::InterruptionEvent;
#[cfg(test)]
pub(crate) use tracker::{TEST_LOCK, record_interruption, reset_session};
