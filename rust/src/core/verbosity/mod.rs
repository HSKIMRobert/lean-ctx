//! Behavioral Verbosity Learning (F10) — adaptive compression recommendations.
//!
//! Learns optimal compression levels from agent behavioral signals.

mod recommender;
mod signals;
mod transcript;

pub use recommender::VerbosityProfile;
#[cfg(test)]
pub(crate) use recommender::recommend_level;
#[cfg(test)]
pub(crate) use signals::extract_signals;
#[cfg(test)]
pub(crate) use transcript::TranscriptEntry;
