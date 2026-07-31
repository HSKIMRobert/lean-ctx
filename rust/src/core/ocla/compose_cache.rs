//! Section-aware cache for `ctx_compose` results.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::core::ocla::cache_types::{CacheKeyBuilder, ComposedContextKey};

#[derive(Clone, Debug)]
struct ComposeRecord {
    source_paths: Vec<PathBuf>,
    source_digests: Vec<String>,
    text: String,
}

/// In-process composition cache. A record is valid only when every source file
/// still has the digest used to build its `ComposedContextKey`.
#[derive(Default)]
pub struct ComposeSectionCache {
    records: Mutex<BTreeMap<(String, String), ComposeRecord>>,
}

impl ComposeSectionCache {
    pub fn check(&self, task: &str, path: &str) -> Option<String> {
        let key = (task.trim().to_string(), path.to_string());
        let record = self.records.lock().ok()?.get(&key)?.clone();
        let source_digests = source_digests(&record.source_paths)?;
        let builder = ComposedContextKey {
            task: key.0,
            path: key.1,
            source_digests,
        };
        (builder.source_digests == record.source_digests).then_some(record.text)
    }

    pub fn record(&self, task: &str, path: &str, text: String) {
        let source_paths = source_paths(path, &text);
        let Some(source_digests) = source_digests(&source_paths) else {
            return;
        };
        let builder = ComposedContextKey {
            task: task.trim().to_string(),
            path: path.to_string(),
            source_digests: source_digests.clone(),
        };
        let _cache_key = builder.cache_key();
        let key = (builder.task, builder.path);
        if let Ok(mut records) = self.records.lock() {
            records.insert(
                key,
                ComposeRecord {
                    source_paths,
                    source_digests,
                    text,
                },
            );
        }
    }
}

pub fn global() -> &'static ComposeSectionCache {
    static CACHE: OnceLock<ComposeSectionCache> = OnceLock::new();
    CACHE.get_or_init(ComposeSectionCache::default)
}

fn source_paths(project_root: &str, text: &str) -> Vec<PathBuf> {
    let root = Path::new(project_root);
    let mut paths = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("File: "))
        .filter_map(|raw| raw.split_whitespace().next())
        .map(|raw| {
            let path = PathBuf::from(raw);
            if path.is_absolute() {
                path
            } else {
                root.join(path)
            }
        })
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

fn source_digests(paths: &[PathBuf]) -> Option<Vec<String>> {
    let mut digests = paths
        .iter()
        .map(|path| {
            std::fs::read(path)
                .ok()
                .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        })
        .collect::<Option<Vec<_>>>()?;
    digests.sort();
    Some(digests)
}

#[cfg(test)]
mod tests {
    use super::ComposeSectionCache;

    #[test]
    fn section_cache_hits_only_while_all_sources_match() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.rs");
        let second = dir.path().join("second.rs");
        std::fs::write(&first, "one").unwrap();
        std::fs::write(&second, "two").unwrap();
        let root = dir.path().to_string_lossy();
        let text = "File: first.rs\nbody one\nFile: second.rs\nbody two".to_string();
        let cache = ComposeSectionCache::default();
        cache.record("task", &root, text.clone());
        assert_eq!(cache.check("task", &root), Some(text));
        std::fs::write(&second, "changed").unwrap();
        assert_eq!(cache.check("task", &root), None);
    }
}
