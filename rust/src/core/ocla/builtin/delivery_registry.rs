//! BuiltinDeliveryRegistry — cross-agent shared read cache.
//!
//! Tracks which files have been read (and compressed) by any agent process.
//! When a second agent requests the same file (same blake3 hash + mtime),
//! a stub is served instead of re-reading and re-compressing, saving tokens.
//!
//! Storage: in-process DashMap keyed by blake3[..12]. The daemon wire_api
//! endpoints expose this store for cross-process coordination via IPC.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;

use crate::core::ocla::traits::{DeliveryRegistry, OclaService};
use crate::core::ocla::types::{
    DeliveryEntry, DeliveryRecord, DeliveryStats, OclaCapability, OclaCapabilityKind,
};
use crate::core::ocla_bus::{self, OclaEvent};

pub struct BuiltinDeliveryRegistry {
    store: DashMap<[u8; 12], DeliveryRecord>,
    stubs_served: AtomicU64,
    tokens_saved: AtomicU64,
    max_entries: usize,
    ttl_secs: u64,
}

impl Default for BuiltinDeliveryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltinDeliveryRegistry {
    pub fn new() -> Self {
        let cfg = crate::core::config::Config::load().ocla.delivery.clone();
        Self {
            store: DashMap::with_capacity(256),
            stubs_served: AtomicU64::new(0),
            tokens_saved: AtomicU64::new(0),
            max_entries: cfg.max_entries,
            ttl_secs: cfg.ttl_minutes * 60,
        }
    }

    #[cfg(test)]
    fn with_limits(max_entries: usize, ttl_secs: u64) -> Self {
        Self {
            store: DashMap::with_capacity(256),
            stubs_served: AtomicU64::new(0),
            tokens_saved: AtomicU64::new(0),
            max_entries,
            ttl_secs,
        }
    }

    fn now_epoch() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn is_expired(&self, record: &DeliveryRecord) -> bool {
        Self::now_epoch().saturating_sub(record.read_at) > self.ttl_secs
    }

    fn evict_expired(&self) {
        let now = Self::now_epoch();
        let ttl = self.ttl_secs;
        self.store
            .retain(|_, record| now.saturating_sub(record.read_at) <= ttl);
    }

    fn evict_oldest_if_full(&self) {
        self.evict_expired();
        if self.store.len() < self.max_entries {
            return;
        }
        let oldest = self
            .store
            .iter()
            .min_by_key(|r| r.read_at)
            .map(|r| *r.key());
        if let Some(key) = oldest {
            self.store.remove(&key);
        }
    }
}

impl OclaService for BuiltinDeliveryRegistry {
    fn capability(&self) -> OclaCapability {
        OclaCapability::available(OclaCapabilityKind::DeliveryRegistry)
    }
}

impl DeliveryRegistry for BuiltinDeliveryRegistry {
    fn check_delivery(&self, blake3: &[u8; 12], mtime: u64) -> Option<DeliveryRecord> {
        let entry = self.store.get(blake3)?;
        if entry.mtime != mtime {
            return None;
        }
        if self.is_expired(entry.value()) {
            drop(entry);
            self.store.remove(blake3);
            return None;
        }
        let record = entry.value().clone();
        drop(entry);

        self.stubs_served.fetch_add(1, Ordering::Relaxed);
        let estimated_tokens = u64::from(record.line_count) * 4;
        self.tokens_saved
            .fetch_add(estimated_tokens, Ordering::Relaxed);

        ocla_bus::emit(OclaEvent::CrossAgentStubServed {
            path: record.path.clone(),
            tokens_saved: estimated_tokens,
            serving_agent: record.agent_id.clone(),
            original_agent: record.conversation_id.clone(),
        });

        Some(record)
    }

    fn record_delivery(&self, entry: DeliveryEntry) {
        self.evict_oldest_if_full();
        let record = DeliveryRecord {
            blake3: entry.blake3,
            path: entry.path,
            line_count: entry.line_count,
            agent_id: entry.agent_id,
            conversation_id: entry.conversation_id,
            read_at: Self::now_epoch(),
            mtime: entry.mtime,
            fresh: true,
        };
        self.store.insert(entry.blake3, record);
    }

    fn delivery_stats(&self) -> DeliveryStats {
        let mut unique_paths = HashSet::new();
        let mut unique_agents = HashSet::new();
        for entry in &self.store {
            unique_paths.insert(entry.path.clone());
            unique_agents.insert(entry.agent_id.clone());
        }
        DeliveryStats {
            total_entries: self.store.len(),
            stubs_served: self.stubs_served.load(Ordering::Relaxed),
            tokens_saved: self.tokens_saved.load(Ordering::Relaxed),
            unique_paths: unique_paths.len(),
            unique_agents: unique_agents.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entry(path: &str, agent: &str, hash: [u8; 12], mtime: u64) -> DeliveryEntry {
        DeliveryEntry {
            blake3: hash,
            path: path.into(),
            line_count: 100,
            agent_id: agent.into(),
            conversation_id: format!("conv-{agent}"),
            mtime,
        }
    }

    #[test]
    fn record_and_check_same_mtime_returns_hit() {
        let reg = BuiltinDeliveryRegistry::new();
        let hash = [1u8; 12];
        reg.record_delivery(test_entry("src/main.rs", "agent-a", hash, 1000));

        let result = reg.check_delivery(&hash, 1000);
        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.path, "src/main.rs");
        assert_eq!(record.agent_id, "agent-a");
    }

    #[test]
    fn different_mtime_returns_miss() {
        let reg = BuiltinDeliveryRegistry::new();
        let hash = [2u8; 12];
        reg.record_delivery(test_entry("src/lib.rs", "agent-b", hash, 1000));

        assert!(reg.check_delivery(&hash, 2000).is_none());
    }

    #[test]
    fn unknown_hash_returns_miss() {
        let reg = BuiltinDeliveryRegistry::new();
        let hash = [3u8; 12];
        assert!(reg.check_delivery(&hash, 1000).is_none());
    }

    #[test]
    fn stats_reflect_entries() {
        let reg = BuiltinDeliveryRegistry::new();
        reg.record_delivery(test_entry("a.rs", "agent-1", [10u8; 12], 100));
        reg.record_delivery(test_entry("b.rs", "agent-2", [11u8; 12], 200));
        reg.record_delivery(test_entry("a.rs", "agent-1", [12u8; 12], 300));

        let stats = reg.delivery_stats();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.unique_paths, 2);
        assert_eq!(stats.unique_agents, 2);
    }

    #[test]
    fn eviction_keeps_store_bounded() {
        let reg = BuiltinDeliveryRegistry::with_limits(100, 3600);
        for i in 0..110 {
            let mut hash = [0u8; 12];
            hash[0] = (i & 0xFF) as u8;
            hash[1] = ((i >> 8) & 0xFF) as u8;
            reg.record_delivery(test_entry("f.rs", "a", hash, i as u64));
        }
        assert!(reg.store.len() <= 100);
    }

    #[test]
    fn stub_served_increments_counters() {
        let reg = BuiltinDeliveryRegistry::new();
        let hash = [4u8; 12];
        reg.record_delivery(test_entry("x.rs", "a", hash, 500));

        let _ = reg.check_delivery(&hash, 500);
        let _ = reg.check_delivery(&hash, 500);

        let stats = reg.delivery_stats();
        assert_eq!(stats.stubs_served, 2);
        assert!(stats.tokens_saved > 0);
    }

    #[test]
    fn expired_entry_returns_miss_and_is_removed() {
        let reg = BuiltinDeliveryRegistry::with_limits(4096, 60);
        let hash = [5u8; 12];
        reg.record_delivery(test_entry("ttl.rs", "agent-ttl", hash, 1000));

        reg.store.get_mut(&hash).unwrap().read_at =
            BuiltinDeliveryRegistry::now_epoch().saturating_sub(120);

        assert!(
            reg.check_delivery(&hash, 1000).is_none(),
            "expired entry must return miss"
        );
        assert_eq!(reg.store.len(), 0, "expired entry must be removed on check");
    }

    #[test]
    fn evict_expired_clears_old_entries() {
        let reg = BuiltinDeliveryRegistry::with_limits(4096, 60);
        reg.record_delivery(test_entry("a.rs", "a1", [20u8; 12], 100));
        reg.record_delivery(test_entry("b.rs", "a2", [21u8; 12], 200));

        let past = BuiltinDeliveryRegistry::now_epoch().saturating_sub(120);
        reg.store.get_mut(&[20u8; 12]).unwrap().read_at = past;
        reg.store.get_mut(&[21u8; 12]).unwrap().read_at = past;

        reg.evict_expired();
        assert_eq!(reg.store.len(), 0);
    }
}
