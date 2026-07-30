//! Delta tracking for append-oriented content streams.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

const PREFIX_LINES: usize = 10;

/// Identifies a tracked append stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct StreamRef {
    pub source_id: String,
    pub stream_type: StreamType,
}

/// Classifies an append stream by its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum StreamType {
    Terminal,
    BuildLog,
    FileWatch,
    Custom,
}

/// Tracks cursors and identity data for one stream generation.
#[derive(Debug, Clone)]
pub(crate) struct StreamState {
    pub generation: u64,
    pub line_cursor: usize,
    pub byte_cursor: usize,
    pub prefix_hash: u64,
    pub last_seen: Instant,
    pub total_lines: usize,
}

/// Minimal update needed to synchronize a stream consumer.
#[derive(Debug, Clone)]
pub(crate) enum StreamDelta {
    /// Content unchanged since last check — deliver nothing.
    Unchanged,
    /// New lines appended — deliver only the new portion.
    Append {
        new_lines: Vec<String>,
        from_line: usize,
    },
    /// Content rotated or replaced — deliver a full snapshot.
    Rotation {
        full_content: Vec<String>,
        reason: String,
    },
    /// Stream expired — client should discard cached state.
    Expired,
}

/// Tracks append streams and computes minimal synchronization deltas.
pub(crate) struct StreamController {
    streams: HashMap<StreamRef, StreamState>,
    max_tracked: usize,
    expiry: Duration,
}

impl StreamController {
    /// Creates a controller with bounded tracking and expiry in seconds.
    pub(crate) fn new(max_tracked: usize, expiry_secs: u64) -> Self {
        Self {
            streams: HashMap::new(),
            max_tracked,
            expiry: Duration::from_secs(expiry_secs),
        }
    }

    /// Compares current content with tracked state and returns its minimal delta.
    pub(crate) fn compute_delta(
        &mut self,
        stream_ref: &StreamRef,
        current_content: &[String],
    ) -> StreamDelta {
        if current_content.is_empty() {
            return StreamDelta::Unchanged;
        }

        let now = Instant::now();
        let Some(state) = self.streams.get_mut(stream_ref) else {
            let state = make_state(1, current_content, now);
            self.ensure_capacity();
            if self.max_tracked > 0 {
                self.streams.insert(stream_ref.clone(), state);
            }
            return rotation(current_content, "first_seen");
        };

        state.last_seen = now;
        if current_content.len() < state.total_lines {
            replace_state(state, current_content, now);
            return rotation(current_content, "truncated");
        }

        let previous_prefix_lines = PREFIX_LINES.min(state.total_lines);
        let current_prefix_hash = compute_prefix_hash(current_content, previous_prefix_lines);
        if current_prefix_hash != state.prefix_hash {
            replace_state(state, current_content, now);
            return rotation(current_content, "prefix_changed");
        }

        match current_content.len().cmp(&state.total_lines) {
            std::cmp::Ordering::Equal => StreamDelta::Unchanged,
            std::cmp::Ordering::Greater => {
                let from_line = state.line_cursor;
                let new_lines = current_content[from_line..].to_vec();
                update_cursors(state, current_content);
                StreamDelta::Append {
                    new_lines,
                    from_line,
                }
            }
            std::cmp::Ordering::Less => {
                replace_state(state, current_content, now);
                rotation(current_content, "truncated")
            }
        }
    }

    /// Removes expired streams and returns the number removed.
    pub(crate) fn gc(&mut self) -> usize {
        let before = self.streams.len();
        let expiry = self.expiry;
        self.streams
            .retain(|_, state| state.last_seen.elapsed() < expiry);
        before - self.streams.len()
    }

    /// Returns the number of actively tracked streams.
    pub(crate) fn tracked_count(&self) -> usize {
        self.streams.len()
    }

    fn ensure_capacity(&mut self) {
        if self.max_tracked == 0 || self.streams.len() < self.max_tracked {
            return;
        }
        if let Some(oldest) = self
            .streams
            .iter()
            .min_by_key(|(_, state)| state.last_seen)
            .map(|(stream_ref, _)| stream_ref.clone())
        {
            self.streams.remove(&oldest);
        }
    }
}

fn compute_prefix_hash(lines: &[String], max_lines: usize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for line in lines.iter().take(max_lines) {
        line.hash(&mut hasher);
    }
    hasher.finish()
}

fn content_bytes(lines: &[String]) -> usize {
    lines.iter().map(String::len).sum()
}

fn make_state(generation: u64, content: &[String], last_seen: Instant) -> StreamState {
    StreamState {
        generation,
        line_cursor: content.len(),
        byte_cursor: content_bytes(content),
        prefix_hash: compute_prefix_hash(content, PREFIX_LINES),
        last_seen,
        total_lines: content.len(),
    }
}

fn replace_state(state: &mut StreamState, content: &[String], last_seen: Instant) {
    *state = make_state(state.generation.saturating_add(1), content, last_seen);
}

fn update_cursors(state: &mut StreamState, content: &[String]) {
    state.line_cursor = content.len();
    state.byte_cursor = content_bytes(content);
    state.prefix_hash = compute_prefix_hash(content, PREFIX_LINES);
    state.total_lines = content.len();
}

fn rotation(content: &[String], reason: &str) -> StreamDelta {
    StreamDelta::Rotation {
        full_content: content.to_vec(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream_ref() -> StreamRef {
        StreamRef {
            source_id: "test-stream".into(),
            stream_type: StreamType::Terminal,
        }
    }

    fn lines(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    fn controller_with(content: &[&str]) -> StreamController {
        let mut controller = StreamController::new(8, 60);
        controller.compute_delta(&stream_ref(), &lines(content));
        controller
    }

    fn rotation(delta: StreamDelta) -> (Vec<String>, String) {
        match delta {
            StreamDelta::Rotation {
                full_content,
                reason,
            } => (full_content, reason),
            other => panic!("expected rotation, got {other:?}"),
        }
    }

    #[test]
    fn test_first_seen_returns_rotation() {
        let content = lines(&["one", "two"]);
        let mut controller = StreamController::new(8, 60);
        let delta = controller.compute_delta(&stream_ref(), &content);
        assert_eq!(rotation(delta), (content, "first_seen".into()));
    }

    #[test]
    fn test_unchanged_content_returns_unchanged() {
        let mut controller = controller_with(&["one", "two"]);
        let delta = controller.compute_delta(&stream_ref(), &lines(&["one", "two"]));
        assert!(matches!(delta, StreamDelta::Unchanged));
    }

    #[test]
    fn test_append_detection() {
        let mut controller = controller_with(&["one", "two"]);
        let delta = controller.compute_delta(&stream_ref(), &lines(&["one", "two", "three"]));
        match delta {
            StreamDelta::Append {
                new_lines,
                from_line,
            } => assert_eq!((new_lines, from_line), (lines(&["three"]), 2)),
            other => panic!("expected append, got {other:?}"),
        }
    }

    #[test]
    fn test_prefix_change_returns_rotation() {
        let mut controller = controller_with(&["one", "two"]);
        let delta = controller.compute_delta(&stream_ref(), &lines(&["changed", "two"]));
        assert_eq!(rotation(delta).1, "prefix_changed");
    }

    #[test]
    fn test_truncation_returns_rotation() {
        let mut controller = controller_with(&["one", "two", "three"]);
        let delta = controller.compute_delta(&stream_ref(), &lines(&["one", "two"]));
        assert_eq!(rotation(delta).1, "truncated");
    }

    #[test]
    fn test_gc_removes_expired_streams() {
        let mut controller = StreamController::new(8, 0);
        controller.compute_delta(&stream_ref(), &lines(&["one"]));
        assert_eq!((controller.gc(), controller.tracked_count()), (1, 0));
    }

    #[test]
    fn test_empty_content_unchanged() {
        let mut controller = StreamController::new(8, 60);
        let delta = controller.compute_delta(&stream_ref(), &[]);
        assert!(matches!(delta, StreamDelta::Unchanged));
        assert_eq!(controller.tracked_count(), 0);
    }
}
