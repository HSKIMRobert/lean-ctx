//! Content-addressed handles for zero-cost re-reads (#1315).
//!
//! Instead of re-delivering file content, return a handle that references
//! the already-delivered content. The handle includes a staleness check
//! so the agent can verify the content hasn't changed.
//!
//! Based on CCF (arXiv 2509.09199): content-addressed hierarchical
//! representations avoid re-processing.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

static HANDLES: Mutex<Option<HandleStore>> = Mutex::new(None);

/// Access the global handle store.
pub(crate) fn global() -> std::sync::MutexGuard<'static, Option<HandleStore>> {
    HANDLES.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A content handle that references previously-delivered content.
#[derive(Debug, Clone)]
pub(crate) struct ContentHandle {
    pub hash: String,
    pub path: String,
    pub line_count: usize,
    pub token_count: usize,
    pub stored_mtime: Option<SystemTime>,
}

impl ContentHandle {
    /// Check if the referenced content is still fresh.
    pub(crate) fn is_fresh(&self) -> bool {
        let Some(stored) = self.stored_mtime else {
            return false;
        };
        std::fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok())
            .is_some_and(|current| current == stored)
    }

    /// Format as a compact reference for the agent.
    pub(crate) fn format_reference(&self) -> String {
        let status = if self.is_fresh() { "fresh" } else { "stale" };
        format!(
            "[handle:{} {} {}L/{}tok {}]",
            &self.hash[..8.min(self.hash.len())],
            self.path,
            self.line_count,
            self.token_count,
            status
        )
    }
}

/// Session-scoped store of content handles.
#[derive(Debug, Clone, Default)]
pub(crate) struct HandleStore {
    handles: HashMap<String, ContentHandle>,
}

impl HandleStore {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Create and store a handle for delivered content.
    pub(crate) fn create_handle(
        &mut self,
        path: &str,
        content: &str,
        line_count: usize,
        token_count: usize,
    ) -> String {
        let hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let short_hash = hash[..12].to_string();

        let stored_mtime = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());

        self.handles.insert(
            short_hash.clone(),
            ContentHandle {
                hash: short_hash.clone(),
                path: path.to_string(),
                line_count,
                token_count,
                stored_mtime,
            },
        );

        short_hash
    }

    /// Look up a handle.
    pub(crate) fn get(&self, handle_id: &str) -> Option<&ContentHandle> {
        self.handles.get(handle_id)
    }

    /// Invalidate handles for a modified file.
    pub(crate) fn invalidate(&mut self, path: &str) {
        self.handles.retain(|_, h| h.path != path);
    }

    pub(crate) fn handle_count(&self) -> usize {
        self.handles.len()
    }

    pub(crate) fn reset(&mut self) {
        self.handles.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_lookup_handle() {
        let mut store = HandleStore::new();
        let id = store.create_handle("/tmp/test.rs", "fn main() {}", 1, 5);
        assert_eq!(id.len(), 12);
        let handle = store.get(&id).unwrap();
        assert_eq!(handle.path, "/tmp/test.rs");
        assert_eq!(handle.line_count, 1);
        assert_eq!(handle.token_count, 5);
    }

    #[test]
    fn same_content_same_handle() {
        let mut store = HandleStore::new();
        let id1 = store.create_handle("/a.rs", "content", 1, 3);
        let id2 = store.create_handle("/b.rs", "content", 1, 3);
        assert_eq!(id1, id2, "same content → same hash handle");
    }

    #[test]
    fn invalidate_removes_file_handles() {
        let mut store = HandleStore::new();
        store.create_handle("/a.rs", "aaa", 1, 3);
        store.create_handle("/b.rs", "bbb", 1, 3);
        assert_eq!(store.handle_count(), 2);
        store.invalidate("/a.rs");
        assert_eq!(store.handle_count(), 1);
    }

    #[test]
    fn format_reference_includes_key_info() {
        let handle = ContentHandle {
            hash: "abcdef123456".to_string(),
            path: "src/lib.rs".to_string(),
            line_count: 100,
            token_count: 400,
            stored_mtime: None,
        };
        let ref_str = handle.format_reference();
        assert!(ref_str.contains("abcdef12"));
        assert!(ref_str.contains("src/lib.rs"));
        assert!(ref_str.contains("100L"));
        assert!(ref_str.contains("400tok"));
        assert!(ref_str.contains("stale"));
    }
}
