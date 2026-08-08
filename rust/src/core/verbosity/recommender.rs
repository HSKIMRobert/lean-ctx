//! Thompson-sampling compression-level recommender from behavioral signals.
//!
//! Maps observed agent behavior to a [`VerbosityProfile`] with deterministic
//! beta-posterior means for reproducible recommendations.

use super::signals::BehaviorSignal;
use crate::core::config::CompressionLevel;

/// Recommended verbosity profile.
#[derive(Debug, Clone)]
pub struct VerbosityProfile {
    /// Recommended compression level.
    pub level: CompressionLevel,
    /// Confidence in the recommendation (0.0–1.0).
    pub confidence: f64,
    /// Reason for the recommendation.
    pub reason: String,
}

/// Recommend a compression level based on behavioral signals.
///
/// Uses the deterministic posterior mean of Thompson-sampling beta priors so
/// identical signal histories always produce identical recommendations.
pub(crate) fn recommend_level(signals: &[BehaviorSignal]) -> VerbosityProfile {
    let mut priors: [(f64, f64); 5] = [(1.0, 1.0), (3.0, 1.0), (2.0, 1.0), (1.5, 1.0), (1.0, 1.5)];

    for signal in signals {
        match signal {
            BehaviorSignal::ReRead { .. } => {
                priors[0].0 += 1.0;
                priors[2].1 += 0.5;
                priors[3].1 += 1.0;
                priors[4].1 += 1.5;
            }
            BehaviorSignal::ModeSwitch { from, to } => {
                if let Some(index) = level_index(from) {
                    priors[index].1 += 0.5;
                }
                if let Some(index) = level_index(to) {
                    priors[index].0 += 0.5;
                }
            }
            BehaviorSignal::FullContentRequest { .. } => {
                priors[0].0 += 1.5;
                priors[2].1 += 0.5;
                priors[3].1 += 1.0;
                priors[4].1 += 1.0;
            }
            BehaviorSignal::TaskComplete { reads_count } => {
                if *reads_count <= 3 {
                    priors[2].0 += 1.0;
                    priors[3].0 += 0.5;
                } else {
                    priors[1].0 += 0.25;
                }
            }
            BehaviorSignal::ExpandFollowUp { .. } => {
                priors[0].0 += 0.5;
                priors[2].1 += 0.5;
                priors[3].1 += 1.0;
            }
        }
    }

    let mut best_idx = 1;
    let mut best_score = 0.0;
    for (index, (alpha, beta)) in priors.iter().enumerate() {
        let score = alpha / (alpha + beta);
        if score >= best_score {
            best_score = score;
            best_idx = index;
        }
    }

    let level = match best_idx {
        0 => CompressionLevel::Off,
        2 => CompressionLevel::Standard,
        3 => CompressionLevel::Max,
        4 => CompressionLevel::Raw,
        _ => CompressionLevel::Lite, // includes 1 (Lite)
    };

    VerbosityProfile {
        level,
        confidence: best_score.clamp(0.0, 1.0),
        reason: format!("Based on {} behavioral signals", signals.len()),
    }
}

fn level_index(level: &str) -> Option<usize> {
    match level.to_ascii_lowercase().as_str() {
        "off" | "full" => Some(0),
        "lite" => Some(1),
        "standard" => Some(2),
        "max" => Some(3),
        "raw" => Some(4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_signals_recommends_lite() {
        assert_eq!(recommend_level(&[]).level, CompressionLevel::Lite);
    }

    #[test]
    fn many_rereads_recommends_less_compression() {
        let signals: Vec<_> = (0..4)
            .map(|_| BehaviorSignal::ReRead {
                path: "a.rs".to_owned(),
                gap_seconds: 1.0,
            })
            .collect();
        assert_eq!(recommend_level(&signals).level, CompressionLevel::Off);
    }

    #[test]
    fn task_complete_boosts_current_level() {
        let profile = recommend_level(&[BehaviorSignal::TaskComplete { reads_count: 2 }]);
        assert_eq!(profile.level, CompressionLevel::Standard);
    }

    #[test]
    fn confidence_between_zero_and_one() {
        let confidence = recommend_level(&[]).confidence;
        assert!((0.0..=1.0).contains(&confidence));
    }
}
