//! Behavioral Verbosity Learning (F10) — adaptive compression recommendations.
//!
//! Learns optimal compression levels from agent behavioral signals.

mod recommender;
mod runtime;
mod signals;
mod transcript;

pub use recommender::VerbosityProfile;
pub(crate) use recommender::recommend_level;
pub(crate) use runtime::recommended_compression;
pub(crate) use runtime::record_tool_call;
#[cfg(test)]
pub(crate) use signals::BehaviorSignal;
pub(crate) use signals::extract_signals;
pub(crate) use transcript::TranscriptEntry;
#[cfg(test)]
pub(crate) use transcript::analyze_transcript;
