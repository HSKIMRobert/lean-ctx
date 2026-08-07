use std::collections::HashMap;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::core::data_dir::lean_ctx_data_dir;
use crate::core::memory_scheduler::fsrs::{
    MemoryState, initial_state, retrievability, update_stability,
};

/// Manages FSRS memory states for all knowledge facts in a project.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnowledgeDecayModel {
    /// Memory states indexed by stable knowledge-fact key.
    pub states: HashMap<String, MemoryState>,
}

impl KnowledgeDecayModel {
    /// Load a project's memory schedule, returning an empty model on failure.
    pub fn load(project_hash: &str) -> Self {
        let Some(path) = Self::path(project_hash) else {
            return Self::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Save a project's memory schedule to disk when its data directory is available.
    pub fn save(&self, project_hash: &str) {
        let Some(path) = Self::path(project_hash) else {
            return;
        };
        let Some(parent) = path.parent() else {
            return;
        };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let Ok(json) = serde_json::to_string_pretty(self) else {
            return;
        };
        let temporary_path = path.with_extension("tmp");
        if std::fs::write(&temporary_path, json).is_ok() {
            let _ = std::fs::rename(temporary_path, path);
        }
    }

    /// Check whether a fact's retrievability is below `threshold`.
    pub fn should_reinject(&self, key: &str, threshold: f64) -> bool {
        match self.states.get(key) {
            Some(state) => retrievability(state, Utc::now()) < threshold,
            None => true,
        }
    }

    /// Record a fact review or successful use with the supplied FSRS rating.
    pub fn record_use(&mut self, key: &str, rating: u8) {
        if let Some(state) = self.states.get_mut(key) {
            update_stability(state, rating);
        } else {
            self.states
                .insert(key.to_string(), initial_state(key.to_string(), rating));
        }
    }

    /// Return fact keys whose retrievability is below `threshold`.
    pub fn stale_facts(&self, threshold: f64) -> Vec<String> {
        let now = Utc::now();
        let mut stale = self
            .states
            .iter()
            .filter(|(_, state)| retrievability(state, now) < threshold)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        stale.sort();
        stale
    }

    /// Return all facts from lowest to highest retrievability.
    pub fn urgency_ranked(&self) -> Vec<(&str, f64)> {
        let now = Utc::now();
        let mut ranked = self
            .states
            .iter()
            .map(|(key, state)| (key.as_str(), retrievability(state, now)))
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| left.1.total_cmp(&right.1).then_with(|| left.0.cmp(right.0)));
        ranked
    }

    /// Resolve the persisted memory-schedule path for a project.
    fn path(project_hash: &str) -> Option<PathBuf> {
        lean_ctx_data_dir().ok().map(|data_dir| {
            data_dir
                .join("knowledge")
                .join(project_hash)
                .join("memory_schedule.json")
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::core::memory_scheduler::decay::KnowledgeDecayModel;
    use crate::core::memory_scheduler::fsrs::initial_state;

    #[test]
    fn should_reinject_true_for_unknown_fact() {
        let model = KnowledgeDecayModel::default();

        assert!(model.should_reinject("unknown", 0.9));
    }

    #[test]
    fn should_reinject_false_immediately_after_use() {
        let mut model = KnowledgeDecayModel::default();
        model.record_use("recent", 3);

        assert!(!model.should_reinject("recent", 0.9));
    }

    #[test]
    fn stale_facts_returns_old_entries() {
        let mut model = KnowledgeDecayModel::default();
        let mut old = initial_state("old".to_string(), 1);
        old.last_review = Utc::now() - Duration::days(30);
        model.states.insert("old".to_string(), old);
        model.record_use("recent", 3);

        assert_eq!(model.stale_facts(0.9), vec!["old".to_string()]);
    }
}
