//! Behavioral Verbosity Learning (F10) — adaptive compression recommendations.
//!
//! Learns optimal compression levels from agent behavioral signals.

mod recommender;
mod signals;
mod transcript;

pub use recommender::VerbosityProfile;
#[allow(unused_imports)]
pub(crate) use recommender::recommend_level;
#[allow(unused_imports)]
pub(crate) use signals::{BehaviorSignal, extract_signals};
#[allow(unused_imports)]
pub(crate) use transcript::{TranscriptAnalysis, TranscriptEntry, analyze_transcript};
