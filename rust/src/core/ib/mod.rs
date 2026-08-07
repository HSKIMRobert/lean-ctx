//! Task-aware Information Bottleneck compression (F1).
//!
//! Extends the existing `information_bottleneck` module with intent classification:
//! instead of requiring explicit query terms, infers them from the agent's session state.

mod intent;
mod relevance;

pub use intent::TaskIntent;
pub(crate) use intent::classify_intent;
pub(crate) use relevance::{RelevanceScore, compute_relevance, intent_query_terms};
