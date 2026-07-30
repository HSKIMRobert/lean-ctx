//! Negative knowledge tracking (#1314).
//!
//! Remembers what the agent already explored and found irrelevant, so
//! lean-ctx can suppress re-exploration of dead-end paths. Based on
//! SARA (ACL 2026): "what should be retrieved next hinges on what has
//! already been inferred from previously retrieved evidence."

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, PoisonError};

static GLOBAL: Mutex<Option<NegativeKnowledge>> = Mutex::new(None);

/// Access the global negative-knowledge tracker.
pub(crate) fn global() -> std::sync::MutexGuard<'static, Option<NegativeKnowledge>> {
    GLOBAL.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Session-scoped negative knowledge store.
#[derive(Debug, Clone, Default)]
pub(crate) struct NegativeKnowledge {
    /// Files confirmed irrelevant to the current task.
    irrelevant_files: HashSet<String>,
    /// Search queries that returned no useful results.
    dead_end_queries: HashSet<String>,
    /// File → set of specific aspects confirmed not present.
    absent_features: HashMap<String, HashSet<String>>,
}

impl NegativeKnowledge {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Mark a file as irrelevant to the current task.
    pub(crate) fn mark_irrelevant(&mut self, path: &str) {
        self.irrelevant_files.insert(path.to_string());
    }

    /// Check if a file has been marked as irrelevant.
    pub(crate) fn is_irrelevant(&self, path: &str) -> bool {
        self.irrelevant_files.contains(path)
    }

    /// Record a dead-end search query.
    pub(crate) fn record_dead_end_query(&mut self, query: &str) {
        self.dead_end_queries.insert(query.to_lowercase());
    }

    /// Check if a similar query was already a dead end.
    pub(crate) fn is_dead_end_query(&self, query: &str) -> bool {
        self.dead_end_queries.contains(&query.to_lowercase())
    }

    /// Record that a specific feature/symbol is absent from a file.
    pub(crate) fn record_absent(&mut self, path: &str, feature: &str) {
        self.absent_features
            .entry(path.to_string())
            .or_default()
            .insert(feature.to_string());
    }

    /// Check if a feature was already confirmed absent from a file.
    pub(crate) fn is_absent(&self, path: &str, feature: &str) -> bool {
        self.absent_features
            .get(path)
            .is_some_and(|features| features.contains(feature))
    }

    /// Invalidate negative knowledge for a file (e.g., after edit).
    pub(crate) fn invalidate(&mut self, path: &str) {
        self.irrelevant_files.remove(path);
        self.absent_features.remove(path);
    }

    /// Summary for diagnostics.
    pub(crate) fn stats(&self) -> (usize, usize, usize) {
        (
            self.irrelevant_files.len(),
            self.dead_end_queries.len(),
            self.absent_features.values().map(HashSet::len).sum(),
        )
    }

    pub(crate) fn reset(&mut self) {
        self.irrelevant_files.clear();
        self.dead_end_queries.clear();
        self.absent_features.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_and_check_irrelevant() {
        let mut nk = NegativeKnowledge::new();
        assert!(!nk.is_irrelevant("src/utils.rs"));
        nk.mark_irrelevant("src/utils.rs");
        assert!(nk.is_irrelevant("src/utils.rs"));
    }

    #[test]
    fn dead_end_queries_case_insensitive() {
        let mut nk = NegativeKnowledge::new();
        nk.record_dead_end_query("DatabasePool");
        assert!(nk.is_dead_end_query("databasepool"));
        assert!(nk.is_dead_end_query("DATABASEPOOL"));
    }

    #[test]
    fn absent_features_per_file() {
        let mut nk = NegativeKnowledge::new();
        nk.record_absent("src/db.rs", "connection_pool");
        assert!(nk.is_absent("src/db.rs", "connection_pool"));
        assert!(!nk.is_absent("src/db.rs", "query_builder"));
        assert!(!nk.is_absent("src/other.rs", "connection_pool"));
    }

    #[test]
    fn invalidate_clears_file_knowledge() {
        let mut nk = NegativeKnowledge::new();
        nk.mark_irrelevant("src/lib.rs");
        nk.record_absent("src/lib.rs", "foo");
        nk.invalidate("src/lib.rs");
        assert!(!nk.is_irrelevant("src/lib.rs"));
        assert!(!nk.is_absent("src/lib.rs", "foo"));
    }

    #[test]
    fn stats_counts() {
        let mut nk = NegativeKnowledge::new();
        nk.mark_irrelevant("a.rs");
        nk.mark_irrelevant("b.rs");
        nk.record_dead_end_query("q1");
        nk.record_absent("c.rs", "x");
        nk.record_absent("c.rs", "y");
        assert_eq!(nk.stats(), (2, 1, 2));
    }
}
