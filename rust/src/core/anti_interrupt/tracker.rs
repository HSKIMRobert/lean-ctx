//! Session-scoped tracking of cognitive interruption events.
//!
//! Records echo repetition, redundant reads, context switches, and related
//! anti-interruption outcomes for metrics and adaptive compression.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Types of cognitive interruption events that lean-ctx prevents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptionEvent {
    /// Agent repeated context already delivered in this session.
    EchoRepetition {
        /// Repeated token count, always ≥ 0.
        tokens: u64,
    },
    /// File re-read without any changes since last read.
    RedundantRead {
        /// Path of the redundantly read file.
        path: String,
    },
    /// Agent jumped between unrelated code areas unnecessarily.
    ContextSwitch {
        /// Previous code area or module label.
        from: String,
        /// New code area or module label.
        to: String,
    },
    /// Tokens wasted on bounce (G7 pattern).
    BounceWaste {
        /// Wasted token count, always ≥ 0.
        tokens: u64,
    },
    /// Knowledge fact injected that the agent already had in context.
    StaleContext {
        /// Stable knowledge-fact key that was already in context.
        fact_key: String,
    },
}

/// Internal event record with timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimestampedEvent {
    event: InterruptionEvent,
    timestamp: DateTime<Utc>,
    prevented: bool,
}

static SESSION_EVENTS: Mutex<Vec<TimestampedEvent>> = Mutex::new(Vec::new());

const MAX_SESSION_EVENTS: usize = 10_000;

fn session_events() -> std::sync::MutexGuard<'static, Vec<TimestampedEvent>> {
    SESSION_EVENTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Record an interruption event (either occurred or was prevented).
pub(crate) fn record_interruption(event: InterruptionEvent, prevented: bool) {
    let mut events = session_events();
    if events.len() >= MAX_SESSION_EVENTS {
        let remove_count = MAX_SESSION_EVENTS / 10;
        events.drain(0..remove_count);
    }
    events.push(TimestampedEvent {
        event,
        timestamp: Utc::now(),
        prevented,
    });
}

/// Get all recorded interruption events for the current session.
pub(crate) fn session_interruptions() -> Vec<(InterruptionEvent, bool)> {
    session_events()
        .iter()
        .map(|event| (event.event.clone(), event.prevented))
        .collect()
}

/// Reset session tracking (called at session start).
pub(crate) fn reset_session() {
    session_events().clear();
}

/// Count prevented interruptions by type.
pub(crate) fn prevented_counts() -> PreventedCounts {
    let events = session_events();
    let mut counts = PreventedCounts::default();
    for event in events.iter().filter(|event| event.prevented) {
        match &event.event {
            InterruptionEvent::EchoRepetition { tokens } => {
                counts.echo_prevented = counts.echo_prevented.saturating_add(*tokens);
            }
            InterruptionEvent::RedundantRead { .. } => counts.redundant_reads_prevented += 1,
            InterruptionEvent::ContextSwitch { .. } => counts.context_switches_prevented += 1,
            InterruptionEvent::BounceWaste { tokens } => {
                counts.bounce_waste_prevented =
                    counts.bounce_waste_prevented.saturating_add(*tokens);
            }
            InterruptionEvent::StaleContext { .. } => counts.stale_context_prevented += 1,
        }
    }
    counts
}

/// Prevented interruption totals, with token-based totals for token-bearing events.
#[derive(Debug, Default)]
pub(crate) struct PreventedCounts {
    /// Echo tokens prevented from being repeated.
    pub echo_prevented: u64,
    /// Redundant file reads prevented.
    pub redundant_reads_prevented: u64,
    /// Unnecessary context switches prevented.
    pub context_switches_prevented: u64,
    /// Bounce-waste tokens prevented.
    pub bounce_waste_prevented: u64,
    /// Stale context injections prevented.
    pub stale_context_prevented: u64,
}

#[cfg(test)]
/// Serializes tests that mutate the global session event store.
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use std::thread;

    use super::{
        InterruptionEvent, TEST_LOCK, prevented_counts, record_interruption, reset_session,
        session_interruptions,
    };

    #[test]
    fn record_and_retrieve_events() {
        let _guard = TEST_LOCK.lock().expect("test lock should be available");
        reset_session();
        record_interruption(
            InterruptionEvent::RedundantRead {
                path: "src/lib.rs".to_string(),
            },
            true,
        );

        let events = session_interruptions();
        assert_eq!(events.len(), 1);
        assert!(events[0].1);
        assert!(matches!(
            events[0].0,
            InterruptionEvent::RedundantRead { .. }
        ));
    }

    #[test]
    fn reset_clears_events() {
        let _guard = TEST_LOCK.lock().expect("test lock should be available");
        reset_session();
        record_interruption(InterruptionEvent::EchoRepetition { tokens: 20 }, false);
        reset_session();

        assert!(session_interruptions().is_empty());
    }

    #[test]
    fn prevented_counts_are_correct() {
        let _guard = TEST_LOCK.lock().expect("test lock should be available");
        reset_session();
        record_interruption(InterruptionEvent::EchoRepetition { tokens: 30 }, true);
        record_interruption(InterruptionEvent::EchoRepetition { tokens: 12 }, false);
        record_interruption(
            InterruptionEvent::ContextSwitch {
                from: "core".to_string(),
                to: "cli".to_string(),
            },
            true,
        );
        record_interruption(InterruptionEvent::BounceWaste { tokens: 8 }, true);
        record_interruption(
            InterruptionEvent::StaleContext {
                fact_key: "decision:format".to_string(),
            },
            true,
        );

        let counts = prevented_counts();
        assert_eq!(counts.echo_prevented, 30);
        assert_eq!(counts.redundant_reads_prevented, 0);
        assert_eq!(counts.context_switches_prevented, 1);
        assert_eq!(counts.bounce_waste_prevented, 8);
        assert_eq!(counts.stale_context_prevented, 1);
    }

    #[test]
    fn concurrent_recording_does_not_panic() {
        let _guard = TEST_LOCK.lock().expect("test lock should be available");
        reset_session();
        let threads: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    for _ in 0..100 {
                        record_interruption(InterruptionEvent::EchoRepetition { tokens: 1 }, true);
                    }
                })
            })
            .collect();

        for handle in threads {
            handle.join().expect("recording thread should not panic");
        }
        assert_eq!(session_interruptions().len(), 1_000);
    }
}
