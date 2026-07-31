//! Process, daemon, and disk cache tiers for generalized delivery entries.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use super::cache_types::{CacheKey, DeliveryEntryV2};

#[derive(Clone, Debug)]
struct MemoryCacheEntry {
    entry: DeliveryEntryV2,
    expires_at: Instant,
}

/// Process-local cache backed by a concurrent map and a fixed TTL.
#[derive(Debug)]
pub struct L1ProcessCache {
    entries: DashMap<CacheKey, MemoryCacheEntry>,
    ttl: Duration,
}

impl L1ProcessCache {
    /// Creates an empty process-local cache with the supplied TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: DashMap::new(),
            ttl,
        }
    }

    /// Returns a live entry and drops expired local entries.
    pub fn get(&self, key: &CacheKey) -> Option<DeliveryEntryV2> {
        let value = self.entries.get(key)?;
        if value.expires_at > Instant::now() {
            return Some(value.entry.clone());
        }
        drop(value);
        self.entries.remove(key);
        None
    }

    /// Inserts or replaces an entry using this tier's TTL.
    pub fn insert(&self, entry: DeliveryEntryV2) {
        let key = entry.key.clone();
        self.entries.insert(
            key,
            MemoryCacheEntry {
                entry,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }

    /// Removes an entry by key.
    pub fn remove(&self, key: &CacheKey) -> Option<DeliveryEntryV2> {
        self.entries.remove(key).map(|(_, value)| value.entry)
    }

    /// Returns the number of entries currently retained by this tier.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether this tier has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Daemon-shared cache backed by a concurrent map and ordered eviction index.
#[derive(Debug)]
pub struct L2DaemonCache {
    entries: DashMap<CacheKey, MemoryCacheEntry>,
    eviction_index: Mutex<BTreeMap<Instant, CacheKey>>,
    max_entries: usize,
    ttl: Duration,
}

impl L2DaemonCache {
    /// Creates an empty daemon cache, clamping its capacity to at least one entry.
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            entries: DashMap::new(),
            eviction_index: Mutex::new(BTreeMap::new()),
            max_entries: max_entries.max(1),
            ttl,
        }
    }

    /// Returns a live entry and refreshes its position in the eviction index.
    pub fn get(&self, key: &CacheKey) -> Option<DeliveryEntryV2> {
        let entry = self.entries.get(key)?.clone();
        if entry.expires_at <= Instant::now() {
            self.remove(key);
            return None;
        }
        self.touch(key.clone());
        Some(entry.entry)
    }

    /// Inserts or replaces an entry, evicting the least-recently-used entry when full.
    pub fn insert(&self, entry: DeliveryEntryV2) -> Option<DeliveryEntryV2> {
        let key = entry.key.clone();
        let mut index = self
            .eviction_index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        remove_index_key(&mut index, &key);
        let previous = self
            .entries
            .insert(
                key.clone(),
                MemoryCacheEntry {
                    entry,
                    expires_at: Instant::now() + self.ttl,
                },
            )
            .map(|value| value.entry);
        let evicted = if previous.is_none() && self.entries.len() > self.max_entries {
            index.pop_first().and_then(|(_, evicted_key)| {
                self.entries
                    .remove(&evicted_key)
                    .map(|(_, value)| value.entry)
            })
        } else {
            None
        };
        let timestamp = unique_instant(&index, Instant::now());
        index.insert(timestamp, key);
        evicted
    }

    /// Removes an entry and its eviction-index record.
    pub fn remove(&self, key: &CacheKey) -> Option<DeliveryEntryV2> {
        let mut index = self
            .eviction_index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        remove_index_key(&mut index, key);
        self.entries.remove(key).map(|(_, value)| value.entry)
    }

    /// Returns the number of entries currently retained by this tier.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether this tier has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn touch(&self, key: CacheKey) {
        let mut index = self
            .eviction_index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        remove_index_key(&mut index, &key);
        let timestamp = unique_instant(&index, Instant::now());
        index.insert(timestamp, key);
    }
}

fn remove_index_key(index: &mut BTreeMap<Instant, CacheKey>, key: &CacheKey) {
    index.retain(|_, indexed_key| indexed_key != key);
}

fn unique_instant(index: &BTreeMap<Instant, CacheKey>, mut timestamp: Instant) -> Instant {
    while index.contains_key(&timestamp) {
        timestamp = timestamp
            .checked_add(Duration::from_nanos(1))
            .unwrap_or(timestamp);
    }
    timestamp
}

/// Serializable manifest record for an entry retained in the disk cache.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistedCacheEntry {
    /// The delivery entry represented by this manifest record.
    pub entry: DeliveryEntryV2,
    /// Time at which the entry was persisted, in Unix epoch milliseconds.
    pub persisted_at_epoch_ms: u64,
}

/// Disk-backed cache manifest with a directory reserved for content blobs.
#[derive(Debug)]
pub struct L3DiskCache {
    root: PathBuf,
    manifest: DashMap<CacheKey, PersistedCacheEntry>,
    blob_directory: PathBuf,
}

impl L3DiskCache {
    /// Opens a disk cache rooted at `root`, creating its manifest and blob directories as needed.
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let blob_directory = root.join("blobs");
        fs::create_dir_all(&blob_directory)?;
        let manifest_path = root.join("manifest.json");
        let entries = if manifest_path.exists() {
            let bytes = fs::read(&manifest_path)?;
            serde_json::from_slice::<Vec<PersistedCacheEntry>>(&bytes).map_err(io::Error::other)?
        } else {
            Vec::new()
        };
        let manifest = DashMap::new();
        for persisted in entries {
            manifest.insert(persisted.entry.key.clone(), persisted);
        }
        Ok(Self {
            root,
            manifest,
            blob_directory,
        })
    }

    /// Validates the manifest on startup: removes entries older than `max_age`
    /// and trims the manifest to `max_bytes` total token budget.
    pub fn startup_validate(&self, max_age: Duration, max_bytes: u64) {
        let now_ms = epoch_ms();
        let max_age_ms = max_age.as_millis() as u64;
        let expired: Vec<CacheKey> = self
            .manifest
            .iter()
            .filter(|entry| now_ms.saturating_sub(entry.persisted_at_epoch_ms) > max_age_ms)
            .map(|entry| entry.key().clone())
            .collect();
        for key in &expired {
            self.manifest.remove(key);
        }
        if max_bytes > 0 {
            let mut entries: Vec<_> = self
                .manifest
                .iter()
                .map(|e| {
                    (
                        e.key().clone(),
                        e.persisted_at_epoch_ms,
                        e.entry.token_count,
                    )
                })
                .collect();
            entries.sort_by_key(|(_, ts, _)| *ts);
            let mut total: u64 = entries.iter().map(|(_, _, t)| *t).sum();
            for (key, _, tokens) in &entries {
                if total <= max_bytes {
                    break;
                }
                total -= tokens;
                self.manifest.remove(key);
            }
        }
        if !expired.is_empty() {
            let _ = self.persist_manifest();
        }
    }

    /// Returns an entry from the persisted manifest without loading its blob.
    pub fn get(&self, key: &CacheKey) -> Option<DeliveryEntryV2> {
        self.manifest
            .get(key)
            .map(|persisted| persisted.entry.clone())
    }

    /// Persists an entry to the manifest and returns any entry it replaced.
    pub fn insert(&self, entry: DeliveryEntryV2) -> io::Result<Option<DeliveryEntryV2>> {
        let key = entry.key.clone();
        let persisted = PersistedCacheEntry {
            entry,
            persisted_at_epoch_ms: epoch_ms(),
        };
        let previous = self.manifest.insert(key, persisted).map(|old| old.entry);
        self.persist_manifest()?;
        Ok(previous)
    }

    /// Removes an entry from the manifest and persists the updated manifest.
    pub fn remove(&self, key: &CacheKey) -> io::Result<Option<DeliveryEntryV2>> {
        let removed = self
            .manifest
            .remove(key)
            .map(|(_, persisted)| persisted.entry);
        if removed.is_some() {
            self.persist_manifest()?;
        }
        Ok(removed)
    }

    /// Returns the cache root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the directory reserved for content-addressed blobs.
    pub fn blob_directory(&self) -> &Path {
        &self.blob_directory
    }

    /// Returns the number of entries present in the manifest.
    pub fn len(&self) -> usize {
        self.manifest.len()
    }

    /// Returns whether the manifest contains no entries.
    pub fn is_empty(&self) -> bool {
        self.manifest.is_empty()
    }

    fn persist_manifest(&self) -> io::Result<()> {
        let mut records = self
            .manifest
            .iter()
            .map(|item| item.value().clone())
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.entry.key.cmp(&right.entry.key));
        let bytes = serde_json::to_vec(&records).map_err(io::Error::other)?;
        let temporary = self.root.join("manifest.json.tmp");
        fs::write(&temporary, bytes)?;
        fs::rename(temporary, self.root.join("manifest.json"))
    }
}

fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ocla::cache_types::{
        AgentHost, CacheIdentity, CacheValidator, ContentHandleRef, DeliveryKind,
    };

    fn entry(name: &str) -> DeliveryEntryV2 {
        DeliveryEntryV2 {
            schema_version: 2,
            key: CacheKey(format!("cache:v1:file_read:{name}")),
            kind: DeliveryKind::FileRead,
            validator: CacheValidator::Immutable,
            handle: ContentHandleRef {
                algorithm: "blake3".into(),
                digest: "d".repeat(64),
                byte_len: 1,
                media_type: "text/plain".into(),
            },
            display_path: None,
            line_count: None,
            token_count: 4,
            producer: CacheIdentity {
                agent_id: "agent".into(),
                conversation_id: "conversation".into(),
                host: AgentHost::Cli,
            },
            created_at_epoch_ms: 0,
            expires_at_epoch_ms: u64::MAX,
        }
    }

    #[test]
    fn l1_expires_entries_using_its_ttl() {
        let cache = L1ProcessCache::new(Duration::ZERO);
        let entry = entry("l1");
        cache.insert(entry.clone());
        assert_eq!(cache.get(&entry.key), None);
        assert!(cache.is_empty());
    }

    #[test]
    fn l2_evicts_the_oldest_entry_at_capacity() {
        let cache = L2DaemonCache::new(1, Duration::from_mins(1));
        let first = entry("first");
        let second = entry("second");
        cache.insert(first.clone());
        assert_eq!(cache.insert(second.clone()), Some(first));
        assert_eq!(cache.get(&second.key), Some(second));
    }

    #[test]
    fn l3_serializes_manifest_entries() {
        let directory = tempfile::tempdir().unwrap();
        let cache = L3DiskCache::open(directory.path()).unwrap();
        let entry = entry("l3");
        cache.insert(entry.clone()).unwrap();
        drop(cache);
        let reopened = L3DiskCache::open(directory.path()).unwrap();
        assert_eq!(reopened.get(&entry.key), Some(entry));
        assert!(reopened.blob_directory().is_dir());
    }

    #[test]
    fn persisted_entry_round_trips() {
        let persisted = PersistedCacheEntry {
            entry: entry("persisted"),
            persisted_at_epoch_ms: 3,
        };
        assert_eq!(
            serde_json::from_str::<PersistedCacheEntry>(
                &serde_json::to_string(&persisted).unwrap()
            )
            .unwrap(),
            persisted
        );
    }

    #[test]
    fn l3_startup_validate_removes_expired_entries() {
        let dir = tempfile::tempdir().unwrap();
        let cache = L3DiskCache::open(dir.path()).unwrap();
        let mut old_entry = entry("old");
        old_entry.created_at_epoch_ms = 1000;
        // persisted_at_epoch_ms set by insert = now, so we need entries
        // that were persisted long ago — override via direct manifest access
        cache.insert(old_entry).unwrap();
        std::thread::sleep(Duration::from_millis(5));
        // With max_age=0, everything should be expired
        cache.startup_validate(Duration::ZERO, u64::MAX);
        assert_eq!(cache.len(), 0, "expired entries must be removed");
    }

    #[test]
    fn l3_startup_validate_trims_to_max_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let cache = L3DiskCache::open(dir.path()).unwrap();
        for i in 0..10 {
            let mut e = entry(&format!("item{i}"));
            e.token_count = 100;
            cache.insert(e).unwrap();
        }
        assert_eq!(cache.len(), 10);
        // max_bytes=500 means only 5 entries of 100 tokens each should survive
        cache.startup_validate(Duration::from_secs(999999), 500);
        assert!(
            cache.len() <= 5,
            "GC must trim to max_bytes budget, got {}",
            cache.len()
        );
    }

    #[test]
    fn l3_manifest_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        {
            let cache = L3DiskCache::open(dir.path()).unwrap();
            cache.insert(entry("persistent")).unwrap();
            assert_eq!(cache.len(), 1);
        }
        let cache = L3DiskCache::open(dir.path()).unwrap();
        assert_eq!(cache.len(), 1, "manifest must persist across reopen");
        assert!(
            cache
                .get(&CacheKey("cache:v1:file_read:persistent".into()))
                .is_some()
        );
    }
}
