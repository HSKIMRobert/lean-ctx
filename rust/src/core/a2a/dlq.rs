use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::agents::AgentRegistry;
use crate::core::ocla::types::{OclaError, OclaResult};

const MAX_ENTRIES: usize = 1_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeadLetter {
    pub id: String,
    pub original_message: String,
    pub target_agent: String,
    pub error: String,
    pub attempts: u8,
    pub first_failed_at: String,
    pub last_failed_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DlqStats {
    pub total: usize,
    pub oldest_age_seconds: u64,
    pub by_target_agent: BTreeMap<String, usize>,
}

#[derive(Clone, Default)]
pub struct DeadLetterQueue {
    entries: Arc<Mutex<Vec<DeadLetter>>>,
}

impl DeadLetterQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&self, letter: DeadLetter) {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if entries.len() == MAX_ENTRIES {
            entries.remove(0);
        }
        entries.push(letter);
    }

    pub fn dequeue(&self, id: &str) -> Option<DeadLetter> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let position = entries.iter().position(|letter| letter.id == id)?;
        Some(entries.remove(position))
    }

    #[must_use]
    pub fn peek_all(&self) -> Vec<DeadLetter> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn retry(&self, id: &str) -> OclaResult<()> {
        let Some(letter) = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .find(|letter| letter.id == id)
            .cloned()
        else {
            return Err(OclaError::InvalidRequest(format!(
                "dead letter not found: {id}"
            )));
        };

        match resend(&letter) {
            Ok(()) => {
                let _ = self.dequeue(id);
                Ok(())
            }
            Err(error) => {
                let mut entries = self
                    .entries
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(current) = entries.iter_mut().find(|current| current.id == id) {
                    current.attempts = current.attempts.saturating_add(1);
                    current.last_failed_at = Utc::now().to_rfc3339();
                }
                Err(error)
            }
        }
    }

    #[must_use]
    pub fn stats(&self) -> DlqStats {
        let entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Utc::now();
        let mut oldest_age_seconds = 0;
        let mut by_target_agent = BTreeMap::new();

        for letter in entries.iter() {
            *by_target_agent
                .entry(letter.target_agent.clone())
                .or_insert(0) += 1;
            if let Ok(failed_at) = DateTime::parse_from_rfc3339(&letter.first_failed_at) {
                let age =
                    u64::try_from((now - failed_at.with_timezone(&Utc)).num_seconds()).unwrap_or(0);
                oldest_age_seconds = oldest_age_seconds.max(age);
            }
        }

        DlqStats {
            total: entries.len(),
            oldest_age_seconds,
            by_target_agent,
        }
    }
}

fn resend(letter: &DeadLetter) -> OclaResult<()> {
    if letter.original_message.trim().is_empty() || letter.target_agent.trim().is_empty() {
        return Err(OclaError::InvalidRequest(
            "dead letter message and target_agent are required".to_string(),
        ));
    }

    AgentRegistry::mutate_locked(|registry| {
        registry.post_message(
            "dead-letter-queue",
            Some(&letter.target_agent),
            "retry",
            &letter.original_message,
        );
    })
    .map(|_| ())
    .map_err(|error| OclaError::InvalidRequest(format!("dead letter retry failed: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn letter(id: &str, target: &str, first_failed_at: &str) -> DeadLetter {
        DeadLetter {
            id: id.to_string(),
            original_message: format!("message-{id}"),
            target_agent: target.to_string(),
            error: "delivery failed".to_string(),
            attempts: 1,
            first_failed_at: first_failed_at.to_string(),
            last_failed_at: first_failed_at.to_string(),
        }
    }

    #[test]
    fn enqueue_dequeue_and_peek_preserve_entries() {
        let queue = DeadLetterQueue::new();
        let item = letter("one", "agent-a", "2026-01-01T00:00:00Z");
        queue.enqueue(item.clone());

        assert_eq!(queue.peek_all(), vec![item.clone()]);
        assert_eq!(queue.dequeue("one"), Some(item));
        assert!(queue.peek_all().is_empty());
        assert!(queue.dequeue("missing").is_none());
    }

    #[test]
    fn enqueue_evicts_oldest_entry_at_capacity() {
        let queue = DeadLetterQueue::new();
        for index in 0..=MAX_ENTRIES {
            queue.enqueue(letter(
                &index.to_string(),
                "agent-a",
                "2026-01-01T00:00:00Z",
            ));
        }

        let entries = queue.peek_all();
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert_eq!(entries.first().map(|entry| entry.id.as_str()), Some("1"));
        assert_eq!(entries.last().map(|entry| entry.id.as_str()), Some("1000"));
    }

    #[test]
    fn stats_report_age_and_target_counts() {
        let queue = DeadLetterQueue::new();
        queue.enqueue(letter("one", "agent-a", "2020-01-01T00:00:00Z"));
        queue.enqueue(letter("two", "agent-a", "2026-01-01T00:00:00Z"));
        queue.enqueue(letter("three", "agent-b", "not-a-timestamp"));

        let stats = queue.stats();
        assert_eq!(stats.total, 3);
        assert!(stats.oldest_age_seconds > 0);
        assert_eq!(stats.by_target_agent.get("agent-a"), Some(&2));
        assert_eq!(stats.by_target_agent.get("agent-b"), Some(&1));
    }

    #[test]
    fn retry_invalid_message_keeps_letter_and_records_attempt() {
        let queue = DeadLetterQueue::new();
        let mut item = letter("one", "agent-a", "2026-01-01T00:00:00Z");
        item.original_message.clear();
        queue.enqueue(item);

        assert!(queue.retry("one").is_err());
        let retained = queue.peek_all();
        assert_eq!(retained[0].attempts, 2);
        assert!(!retained[0].last_failed_at.is_empty());
    }

    #[test]
    fn retry_resends_and_removes_letter() {
        let _isolated = crate::core::data_dir::isolated_data_dir();
        let queue = DeadLetterQueue::new();
        queue.enqueue(letter("one", "agent-a", "2026-01-01T00:00:00Z"));

        queue.retry("one").expect("retry succeeds");
        assert!(queue.peek_all().is_empty());

        let registry = AgentRegistry::load().expect("registry persisted");
        assert_eq!(registry.scratchpad[0].to_agent.as_deref(), Some("agent-a"));
        assert_eq!(registry.scratchpad[0].message, "message-one");
    }
}
