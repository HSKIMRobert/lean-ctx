//! Session-scoped cache for deduplicating compressed tool results.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const MAX_ENTRIES: usize = 256;
const TTL_SECS: u64 = 1800;
const FIRST_LINE_MAX: usize = 120;

/// Thread-safe cache of compressed tool results for one proxy session.
pub struct ToolResultCache {
    entries: DashMap<u64, CacheEntry>,
    current_turn: AtomicU64,
    // TODO(#1354): remove dead code or implement
    created_at: Instant,
}

struct CacheEntry {
    turn_seen: u64,
    token_count: usize,
    first_line: String,
    ccr_handle: Option<String>,
    inserted_at: Instant,
}

/// A prior tool result that can be represented by a compact stub.
pub struct DedupHit {
    pub turn_seen: u64,
    pub tokens_saved: usize,
    pub stub: String,
}

impl ToolResultCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
            current_turn: AtomicU64::new(0),
            created_at: Instant::now(),
        }
    }

    /// Check whether this exact content was already compressed in this session.
    #[must_use]
    pub fn check(&self, tool_name: &str, content: &str) -> Option<DedupHit> {
        let key = cache_key(tool_name, content);
        let entry = self.entries.get(&key)?;
        if entry.inserted_at.elapsed().as_secs() > TTL_SECS {
            drop(entry);
            self.entries.remove(&key);
            return None;
        }

        let mut stub = format!(
            "[unchanged since turn {} — {} tokens elided]\n{}...",
            entry.turn_seen, entry.token_count, entry.first_line
        );
        if let Some(ccr_handle) = &entry.ccr_handle {
            stub.push_str(&format!("\n[lean-ctx: full content at {ccr_handle}]"));
        }
        Some(DedupHit {
            turn_seen: entry.turn_seen,
            tokens_saved: entry.token_count,
            stub,
        })
    }

    /// Insert a tool result after compression.
    pub fn insert(
        &self,
        tool_name: &str,
        content: &str,
        token_count: usize,
        ccr_handle: Option<String>,
    ) {
        if self.entries.len() >= MAX_ENTRIES
            && let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|entry| entry.inserted_at)
                .map(|entry| *entry.key())
        {
            self.entries.remove(&oldest_key);
        }

        self.entries.insert(
            cache_key(tool_name, content),
            CacheEntry {
                turn_seen: self.turn(),
                token_count,
                first_line: preview_line(content),
                ccr_handle,
                inserted_at: Instant::now(),
            },
        );
    }

    /// Advance the session's API-request turn counter.
    pub fn advance_turn(&self) {
        self.current_turn.fetch_add(1, Ordering::Relaxed);
    }

    /// Return the current session turn number.
    #[must_use]
    pub fn turn(&self) -> u64 {
        self.current_turn.load(Ordering::Relaxed)
    }
}

impl Default for ToolResultCache {
    fn default() -> Self {
        Self::new()
    }
}

fn cache_key(tool_name: &str, content: &str) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(tool_name.as_bytes());
    hasher.update(b"\0");
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&hash.as_bytes()[..8]);
    u64::from_le_bytes(bytes)
}

fn preview_line(content: &str) -> String {
    content
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(FIRST_LINE_MAX)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{CacheEntry, FIRST_LINE_MAX, MAX_ENTRIES, ToolResultCache, cache_key};
    use std::time::{Duration, Instant};

    #[test]
    fn insert_then_check_returns_hit() {
        let cache = ToolResultCache::new();
        cache.insert("ctx_read", "source contents", 42, None);

        let hit = cache
            .check("ctx_read", "source contents")
            .expect("cache hit");
        assert_eq!(hit.turn_seen, 0);
        assert_eq!(hit.tokens_saved, 42);
    }

    #[test]
    fn check_miss_returns_none() {
        let cache = ToolResultCache::new();
        assert!(cache.check("ctx_read", "new contents").is_none());
    }

    #[test]
    fn eviction_at_max_entries() {
        let cache = ToolResultCache::new();
        for index in 0..MAX_ENTRIES {
            cache.insert("ctx_read", &format!("content-{index}"), 1, None);
        }
        cache.insert("ctx_read", "newest", 1, None);

        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        assert!(cache.check("ctx_read", "content-0").is_none());
        assert!(cache.check("ctx_read", "newest").is_some());
    }

    #[test]
    fn ttl_expiry_returns_none() {
        let cache = ToolResultCache::new();
        let key = cache_key("ctx_read", "expired");
        cache.entries.insert(
            key,
            CacheEntry {
                turn_seen: 0,
                token_count: 1,
                first_line: "expired".to_string(),
                ccr_handle: None,
                inserted_at: Instant::now()
                    .checked_sub(Duration::from_secs(1801))
                    .unwrap(),
            },
        );

        assert!(cache.check("ctx_read", "expired").is_none());
        assert!(!cache.entries.contains_key(&key));
    }

    #[test]
    fn advance_turn_increments() {
        let cache = ToolResultCache::new();
        cache.advance_turn();
        cache.advance_turn();
        assert_eq!(cache.turn(), 2);
    }

    #[test]
    fn different_tool_names_produce_different_keys() {
        assert_ne!(
            cache_key("ctx_read", "content"),
            cache_key("ctx_shell", "content")
        );
    }

    #[test]
    fn stub_format_includes_turn_and_tokens() {
        let cache = ToolResultCache::new();
        cache.advance_turn();
        cache.insert(
            "ctx_read",
            "first line\nremaining",
            17,
            Some("ccr://result".to_string()),
        );

        let hit = cache
            .check("ctx_read", "first line\nremaining")
            .expect("cache hit");
        assert_eq!(
            hit.stub,
            "[unchanged since turn 1 — 17 tokens elided]\nfirst line...\n[lean-ctx: full content at ccr://result]"
        );
    }

    #[test]
    fn preview_line_is_character_limited() {
        let cache = ToolResultCache::new();
        let content = "x".repeat(FIRST_LINE_MAX + 1);
        cache.insert("ctx_read", &content, 1, None);

        let hit = cache.check("ctx_read", &content).expect("cache hit");
        assert_eq!(hit.stub.matches('x').count(), FIRST_LINE_MAX);
    }
}
