//! Background trajectory prefetch — warms OS page cache and session cache.

use std::path::Path;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::core::cache::SessionCache;
use crate::core::cognitive_gate::full_science_enabled;
use crate::core::io_boundary::read_file_lossy;

const MAX_WARM_FILE_BYTES: usize = 50 * 1024;

/// Spawn background threads to warm up to three predicted files.
pub(crate) fn warm_predictions(predictions: &[String], cache: Option<&Arc<RwLock<SessionCache>>>) {
    if !full_science_enabled() {
        return;
    }
    spawn_warm_threads(predictions, cache);
}

fn spawn_warm_threads(predictions: &[String], cache: Option<&Arc<RwLock<SessionCache>>>) {
    for path in predictions.iter().take(3) {
        let path = path.clone();
        let cache_clone = cache.cloned();
        std::thread::spawn(move || {
            warm_single_file(&path, cache_clone.as_ref());
        });
    }
}

fn warm_single_file(path: &str, cache: Option<&Arc<RwLock<SessionCache>>>) {
    let path_obj = Path::new(path);
    if !path_obj.is_file() {
        return;
    }
    if let Ok(meta) = std::fs::metadata(path) {
        if !meta.is_file() || meta.len() > MAX_WARM_FILE_BYTES as u64 {
            return;
        }
    }

    if let Some(cache_arc) = cache
        && let Ok(guard) = cache_arc.try_read()
        && guard.get(path).is_some()
    {
        tracing::debug!("[prefetch] skip (session cached): {path}");
        return;
    }

    let Ok(content) = read_file_lossy(path) else {
        return;
    };
    tracing::debug!("[prefetch] warmed cache: {path}");

    if let Some(cache_arc) = cache
        && let Ok(mut guard) = cache_arc.try_write()
        && guard.get(path).is_none()
    {
        guard.store(path, &content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn prefetch_warm_skips_missing_file() {
        let cache = Arc::new(RwLock::new(SessionCache::new()));
        spawn_warm_threads(&["/no/such/prefetch-file-xyz.rs".to_string()], Some(&cache));
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(
            cache
                .blocking_read()
                .get("/no/such/prefetch-file-xyz.rs")
                .is_none()
        );
    }

    #[test]
    fn prefetch_warm_stores_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("warm_me.rs");
        std::fs::write(&file_path, "fn warm_me() {}").unwrap();
        let path = file_path.to_string_lossy().into_owned();

        let cache = Arc::new(RwLock::new(SessionCache::new()));
        let paths = [path.clone()];
        spawn_warm_threads(&paths, Some(&cache));

        // Background thread — brief spin until store lands.
        for _ in 0..50 {
            if cache.blocking_read().get(&path).is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(cache.blocking_read().get(&path).is_some());
    }
}
