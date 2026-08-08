//! FSRS-based Context Memory Scheduler (F5).
//!
//! Models knowledge retention using the Free Spaced Repetition Scheduler algorithm.
//! Determines which facts need re-injection based on their retrievability score.

mod decay;
mod fsrs;

pub use fsrs::MemoryState;
#[cfg(test)]
pub(crate) use fsrs::update_stability;
pub(crate) use fsrs::{initial_state, retrievability};
