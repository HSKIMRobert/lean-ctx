use chrono::Utc;
use std::path::PathBuf;

use super::ranking::hash_project_root;
use super::types::{ConsolidatedInsight, KnowledgeFact, ProjectKnowledge, ProjectPattern};
use crate::core::memory_policy::MemoryPolicy;

fn knowledge_dir(project_hash: &str) -> Result<PathBuf, String> {
    Ok(crate::core::data_dir::lean_ctx_data_dir()?
        .join("knowledge")
        .join(project_hash))
}

impl ProjectKnowledge {
    pub fn save(&self) -> Result<(), String> {
        let dir = knowledge_dir(&self.project_hash)?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let path = dir.join("knowledge.json");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }

    pub fn load(project_root: &str) -> Option<Self> {
        let hash = hash_project_root(project_root);
        let dir = knowledge_dir(&hash).ok()?;
        let path = dir.join("knowledge.json");

        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(k) = serde_json::from_str::<Self>(&content) {
                return Some(k);
            }
        }

        let old_hash = crate::core::project_hash::hash_path_only(project_root);
        if old_hash != hash {
            crate::core::project_hash::migrate_if_needed(&old_hash, &hash, project_root);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mut k) = serde_json::from_str::<Self>(&content) {
                    k.project_hash = hash;
                    let _ = k.save();
                    return Some(k);
                }
            }
        }

        None
    }

    pub fn load_or_create(project_root: &str) -> Self {
        Self::load(project_root).unwrap_or_else(|| Self::new(project_root))
    }

    /// Migrates legacy knowledge that was accidentally stored under an empty project_root ("")
    /// into the given `target_root`. Keeps a timestamped backup of the legacy file.
    pub fn migrate_legacy_empty_root(
        target_root: &str,
        policy: &MemoryPolicy,
    ) -> Result<bool, String> {
        if target_root.trim().is_empty() {
            return Ok(false);
        }

        let Some(legacy) = Self::load("") else {
            return Ok(false);
        };

        if !legacy.project_root.trim().is_empty() {
            return Ok(false);
        }
        if legacy.facts.is_empty() && legacy.patterns.is_empty() && legacy.history.is_empty() {
            return Ok(false);
        }

        let mut target = Self::load_or_create(target_root);

        fn fact_key(f: &KnowledgeFact) -> String {
            format!(
                "{}|{}|{}|{}|{}",
                f.category, f.key, f.value, f.source_session, f.created_at
            )
        }
        fn pattern_key(p: &ProjectPattern) -> String {
            format!(
                "{}|{}|{}|{}",
                p.pattern_type, p.description, p.source_session, p.created_at
            )
        }
        fn history_key(h: &ConsolidatedInsight) -> String {
            format!(
                "{}|{}|{}",
                h.summary,
                h.from_sessions.join(","),
                h.timestamp
            )
        }

        let mut seen_facts: std::collections::HashSet<String> =
            target.facts.iter().map(fact_key).collect();
        for f in legacy.facts {
            if seen_facts.insert(fact_key(&f)) {
                target.facts.push(f);
            }
        }

        let mut seen_patterns: std::collections::HashSet<String> =
            target.patterns.iter().map(pattern_key).collect();
        for p in legacy.patterns {
            if seen_patterns.insert(pattern_key(&p)) {
                target.patterns.push(p);
            }
        }

        let mut seen_history: std::collections::HashSet<String> =
            target.history.iter().map(history_key).collect();
        for h in legacy.history {
            if seen_history.insert(history_key(&h)) {
                target.history.push(h);
            }
        }

        target.facts.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.confidence.total_cmp(&a.confidence))
        });
        if target.facts.len() > policy.knowledge.max_facts {
            target.facts.truncate(policy.knowledge.max_facts);
        }
        target
            .patterns
            .sort_by_key(|x| std::cmp::Reverse(x.created_at));
        if target.patterns.len() > policy.knowledge.max_patterns {
            target.patterns.truncate(policy.knowledge.max_patterns);
        }
        target
            .history
            .sort_by_key(|x| std::cmp::Reverse(x.timestamp));
        if target.history.len() > policy.knowledge.max_history {
            target.history.truncate(policy.knowledge.max_history);
        }

        target.updated_at = Utc::now();
        target.save()?;

        let legacy_hash = crate::core::project_hash::hash_path_only("");
        let legacy_dir = knowledge_dir(&legacy_hash)?;
        let legacy_path = legacy_dir.join("knowledge.json");
        if legacy_path.exists() {
            let ts = Utc::now().format("%Y%m%d-%H%M%S");
            let backup = legacy_dir.join(format!("knowledge.legacy-empty-root.{ts}.json"));
            std::fs::rename(&legacy_path, &backup).map_err(|e| e.to_string())?;
        }

        Ok(true)
    }
}
