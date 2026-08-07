//! Anti-Interruption Score (F7) — cognitive impact metrics beyond token savings.
//!
//! Tracks interruption events (echo repetitions, redundant reads, context switches)
//! and computes a session-level score measuring cognitive interruptions prevented.
#![allow(unreachable_pub)]

mod metrics;
mod tracker;

#[allow(unused_imports)]
pub(crate) use metrics::{CognitiveImpactReport, compute_impact};
pub use tracker::InterruptionEvent;
#[allow(unused_imports)]
pub(crate) use tracker::{record_interruption, reset_session, session_interruptions};
