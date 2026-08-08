//! Behavioral signals extracted from agent compression transcripts.
//!
//! Detects re-reads, mode switches, expand follow-ups, and task completion
//! patterns that indicate over- or under-compression.

use super::transcript::TranscriptEntry;
use chrono::Duration;
use std::collections::HashMap;

/// Behavioral signals that indicate compression quality.
#[derive(Debug, Clone)]
pub enum BehaviorSignal {
    /// Agent re-read the same file, suggesting compression may have been too aggressive.
    ReRead {
        /// Re-read path.
        path: String,
        /// Seconds since the preceding read.
        gap_seconds: f64,
    },
    /// Agent asked for a different mode after reading, indicating the mode was wrong.
    ModeSwitch {
        /// Previous mode.
        from: String,
        /// Newly requested mode.
        to: String,
    },
    /// Agent explicitly requested full content, suggesting compression was too aggressive.
    FullContentRequest {
        /// Requested path.
        path: String,
    },
    /// Agent completed the task without re-reads, suggesting compression was appropriate.
    TaskComplete {
        /// Number of reads used to complete the task.
        reads_count: usize,
    },
    /// Agent followed up with `ctx_expand`, indicating more detail was needed.
    ExpandFollowUp {
        /// Expanded path.
        path: String,
    },
}

/// Extract behavioral signals from a transcript.
pub(crate) fn extract_signals(entries: &[TranscriptEntry]) -> Vec<BehaviorSignal> {
    let Some(newest) = entries.iter().map(|entry| entry.timestamp).max() else {
        return Vec::new();
    };
    let cutoff = newest - Duration::minutes(10);
    let recent: Vec<&TranscriptEntry> = entries
        .iter()
        .filter(|entry| entry.timestamp >= cutoff)
        .collect();

    let mut signals = Vec::new();
    let mut last_read_by_target: HashMap<&str, &TranscriptEntry> = HashMap::new();

    for (index, entry) in recent.iter().enumerate() {
        if entry.tool == "ctx_read" {
            if let Some(previous) = last_read_by_target.get(entry.target.as_str()) {
                let gap = entry.timestamp.signed_duration_since(previous.timestamp);
                if gap >= Duration::zero() && gap <= Duration::seconds(60) {
                    signals.push(BehaviorSignal::ReRead {
                        path: entry.target.clone(),
                        gap_seconds: gap.num_milliseconds() as f64 / 1_000.0,
                    });
                }
            }
            last_read_by_target.insert(entry.target.as_str(), entry);
        }

        if index == 0 {
            continue;
        }
        let previous = recent[index - 1];
        if entry.tool == "ctx_read"
            && previous.tool == "ctx_read"
            && entry.target == previous.target
            && entry.compression_level != previous.compression_level
        {
            signals.push(BehaviorSignal::ModeSwitch {
                from: previous.compression_level.clone(),
                to: entry.compression_level.clone(),
            });
            if entry.compression_level.eq_ignore_ascii_case("full")
                && !previous.compression_level.eq_ignore_ascii_case("full")
            {
                signals.push(BehaviorSignal::FullContentRequest {
                    path: entry.target.clone(),
                });
            }
        }
        if entry.tool == "ctx_expand" && previous.tool == "ctx_read" {
            signals.push(BehaviorSignal::ExpandFollowUp {
                path: entry.target.clone(),
            });
        }
    }

    signals
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn entry(target: &str, level: &str, seconds: i64) -> TranscriptEntry {
        TranscriptEntry {
            tool: "ctx_read".to_owned(),
            target: target.to_owned(),
            compression_level: level.to_owned(),
            response_tokens: 10,
            timestamp: Utc.timestamp_opt(seconds, 0).single().expect("valid time"),
        }
    }

    #[test]
    fn reread_detected_within_60s() {
        let entries = [entry("a.rs", "lite", 0), entry("a.rs", "lite", 60)];
        assert!(
            extract_signals(&entries)
                .iter()
                .any(|signal| matches!(signal, BehaviorSignal::ReRead { .. }))
        );
    }

    #[test]
    fn mode_switch_detected() {
        let entries = [entry("a.rs", "max", 0), entry("a.rs", "full", 1)];
        let signals = extract_signals(&entries);
        assert!(signals.iter().any(|signal| matches!(
            signal,
            BehaviorSignal::ModeSwitch { from, to } if from == "max" && to == "full"
        )));
        assert!(
            signals
                .iter()
                .any(|signal| matches!(signal, BehaviorSignal::FullContentRequest { .. }))
        );
    }

    #[test]
    fn no_signals_from_unique_reads() {
        let entries = [entry("a.rs", "lite", 0), entry("b.rs", "lite", 1)];
        assert!(extract_signals(&entries).is_empty());
    }
}
