use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use super::bm25_index::BM25Index;

const DEFAULT_TTL_SECS: u64 = 60;

pub struct Bm25CacheEntry {
    pub root: PathBuf,
    pub index: Arc<BM25Index>,
    pub loaded_at: Instant,
}

impl Bm25CacheEntry {
    pub fn is_fresh(&self) -> bool {
        self.loaded_at.elapsed().as_secs() < ttl_secs()
    }
}

fn ttl_secs() -> u64 {
    std::env::var("LEAN_CTX_BM25_CACHE_TTL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_TTL_SECS)
}

pub type SharedBm25Cache = std::sync::Arc<std::sync::Mutex<Option<Bm25CacheEntry>>>;

/// Get the BM25 index from cache if available and fresh, otherwise load/build,
/// cache it, and return. Uses Arc to avoid cloning the entire index.
pub fn get_or_load(cache: &SharedBm25Cache, root: &Path) -> Arc<BM25Index> {
    {
        let guard = cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(ref entry) = *guard {
            if entry.root == root && entry.is_fresh() {
                return Arc::clone(&entry.index);
            }
        }
    }

    let index = Arc::new(BM25Index::load_or_build_fast(root));

    let mut guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = Some(Bm25CacheEntry {
        root: root.to_path_buf(),
        index: Arc::clone(&index),
        loaded_at: Instant::now(),
    });

    index
}

/// Get index from cache (fresh or stale), triggering background rebuild if stale.
/// Returns None only if no cache entry exists at all.
pub fn get_or_background(cache: &SharedBm25Cache, root: &Path) -> Option<Arc<BM25Index>> {
    let guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let entry = guard.as_ref()?;
    if entry.root != root {
        return None;
    }

    let idx = Arc::clone(&entry.index);

    if !entry.is_fresh() {
        let root_str = root.to_string_lossy().to_string();
        let cache_clone = cache.clone();
        let root_clone = root.to_path_buf();
        std::thread::spawn(move || {
            let rebuilt = BM25Index::load_or_build(&root_clone);
            let mut g = cache_clone
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *g = Some(Bm25CacheEntry {
                root: root_clone,
                index: Arc::new(rebuilt),
                loaded_at: Instant::now(),
            });
            tracing::debug!("[bm25_cache: background refresh done for {root_str}]");
        });
    }

    Some(idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn fresh_cache_returns_same_instance() {
        let cache: SharedBm25Cache = Arc::new(std::sync::Mutex::new(None));
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();

        let idx1 = get_or_load(&cache, root);
        assert!(idx1.doc_count > 0);

        let idx2 = get_or_load(&cache, root);
        assert_eq!(idx1.doc_count, idx2.doc_count);
    }

    #[test]
    fn different_root_invalidates() {
        let cache: SharedBm25Cache = Arc::new(std::sync::Mutex::new(None));
        let tmp1 = tempfile::tempdir().unwrap();
        let tmp2 = tempfile::tempdir().unwrap();
        std::fs::write(tmp1.path().join("a.rs"), "fn a() {}\n").unwrap();
        std::fs::write(tmp2.path().join("b.rs"), "fn b() {}\n").unwrap();

        let _ = get_or_load(&cache, tmp1.path());
        let idx2 = get_or_load(&cache, tmp2.path());

        let guard = cache.lock().unwrap();
        let entry = guard.as_ref().unwrap();
        assert_eq!(entry.root, tmp2.path());
        assert_eq!(entry.index.doc_count, idx2.doc_count);
    }

    #[test]
    fn get_or_background_returns_none_on_empty() {
        let cache: SharedBm25Cache = Arc::new(std::sync::Mutex::new(None));
        let tmp = tempfile::tempdir().unwrap();
        assert!(get_or_background(&cache, tmp.path()).is_none());
    }
}
