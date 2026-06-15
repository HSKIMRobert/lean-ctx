use crate::core::events::LeanCtxEvent;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

pub(super) struct EventTail {
    path: PathBuf,
    offset: u64,
}

impl EventTail {
    pub(super) fn new() -> Self {
        let base = crate::core::paths::state_dir()
            .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".lean-ctx"));
        let path = base.join("events.jsonl");
        let offset = std::fs::metadata(&path).map_or(0, |m| m.len());
        Self { path, offset }
    }

    /// Read the last `n` events already in the log so `watch` shows recent
    /// history immediately instead of a blank screen when started while idle
    /// (#560). The internal offset is advanced to EOF so the subsequent
    /// `poll()` stream continues seamlessly without re-emitting these events.
    pub(super) fn backfill(&mut self, n: usize) -> Vec<LeanCtxEvent> {
        if n == 0 {
            return Vec::new();
        }
        let Ok(file) = std::fs::File::open(&self.path) else {
            return Vec::new();
        };
        let meta_len = file.metadata().map_or(0, |m| m.len());
        let reader = BufReader::new(&file);
        // Bounded ring buffer: keep only the last `n` parsed events so memory
        // stays O(n) regardless of how large events.jsonl has grown.
        let mut recent: VecDeque<LeanCtxEvent> = VecDeque::with_capacity(n + 1);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<LeanCtxEvent>(&line) {
                if recent.len() == n {
                    recent.pop_front();
                }
                recent.push_back(event);
            }
        }
        // Continue the live stream from the EOF we just observed.
        self.offset = meta_len;
        recent.into_iter().collect()
    }

    pub(super) fn poll(&mut self) -> Vec<LeanCtxEvent> {
        let Ok(mut file) = std::fs::File::open(&self.path) else {
            return Vec::new();
        };
        let meta_len = file.metadata().map_or(0, |m| m.len());
        if meta_len < self.offset {
            self.offset = 0;
        }
        if meta_len == self.offset {
            return Vec::new();
        }

        let _ = file.seek(SeekFrom::Start(self.offset));
        let reader = BufReader::new(&file);
        let mut events = Vec::new();
        let mut bytes_read: u64 = 0;

        for line in reader.lines() {
            let Ok(line) = line else { break };
            bytes_read += line.len() as u64 + 1; // +1 for newline
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<LeanCtxEvent>(&line) {
                events.push(event);
            }
        }

        self.offset += bytes_read;
        events
    }

    #[cfg(test)]
    fn with_path(path: PathBuf) -> Self {
        let offset = std::fs::metadata(&path).map_or(0, |m| m.len());
        Self { path, offset }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::EventKind;
    use std::io::Write;

    fn make_event(i: usize) -> LeanCtxEvent {
        LeanCtxEvent {
            id: i as u64,
            timestamp: String::new(),
            kind: EventKind::ToolCall {
                tool: format!("tool-{i}"),
                tokens_original: 0,
                tokens_saved: 0,
                mode: None,
                duration_ms: 0,
                path: None,
            },
        }
    }

    fn append_events(path: &std::path::Path, range: std::ops::Range<usize>) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        for i in range {
            writeln!(f, "{}", serde_json::to_string(&make_event(i)).unwrap()).unwrap();
        }
    }

    fn tool_name(ev: &LeanCtxEvent) -> &str {
        match &ev.kind {
            EventKind::ToolCall { tool, .. } => tool,
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn backfill_returns_last_n_events_and_advances_offset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        append_events(&path, 0..50);

        let mut tail = EventTail::with_path(path.clone());
        let recent = tail.backfill(20);
        assert_eq!(recent.len(), 20, "backfill keeps only the last n events");
        // Oldest kept is event #30, newest is #49.
        assert_eq!(tool_name(recent.first().unwrap()), "tool-30");
        assert_eq!(tool_name(recent.last().unwrap()), "tool-49");
        // Offset is at EOF -> no re-emission of the backfilled tail.
        assert!(
            tail.poll().is_empty(),
            "poll after backfill must not repeat"
        );
    }

    #[test]
    fn backfill_then_poll_streams_only_new_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        append_events(&path, 0..5);

        let mut tail = EventTail::with_path(path.clone());
        assert_eq!(tail.backfill(20).len(), 5, "fewer than n -> return all");

        // Append two new events; only those must surface on the next poll.
        append_events(&path, 5..7);
        let streamed = tail.poll();
        assert_eq!(streamed.len(), 2, "only the two appended events stream");
        assert_eq!(tool_name(streamed.first().unwrap()), "tool-5");
    }

    #[test]
    fn backfill_on_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.jsonl");
        let mut tail = EventTail::with_path(path);
        assert!(tail.backfill(20).is_empty());
    }
}
