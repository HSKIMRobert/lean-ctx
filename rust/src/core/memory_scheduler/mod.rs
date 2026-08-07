//! FSRS-based Context Memory Scheduler (F5).
//!
//! Models knowledge retention using the Free Spaced Repetition Scheduler algorithm.
//! Determines which facts need re-injection based on their retrievability score.
#![allow(unreachable_pub)]

mod decay;
mod fsrs;

#[allow(unused_imports)]
pub(crate) use decay::KnowledgeDecayModel;
pub use fsrs::MemoryState;
#[allow(unused_imports)]
pub(crate) use fsrs::{optimal_interval, retrievability, update_stability};
