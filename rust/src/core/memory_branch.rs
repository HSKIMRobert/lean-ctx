use crate::core::knowledge::{KnowledgeFact, ProjectKnowledge};
use std::collections::HashMap;

#[allow(dead_code)]
pub(crate) struct MemoryBranch {
    #[allow(dead_code)]
    conversation_id: String,
    overrides: HashMap<String, KnowledgeFact>,
    deletions: Vec<String>,
    #[allow(dead_code)]
    created_at: chrono::DateTime<chrono::Utc>,
}

#[allow(dead_code)]
impl MemoryBranch {
    pub(crate) fn new(conversation_id: &str) -> Self {
        Self {
            conversation_id: conversation_id.to_string(),
            overrides: HashMap::new(),
            deletions: Vec::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub(crate) fn override_fact(&mut self, fact: KnowledgeFact) {
        self.deletions.retain(|k| k != &fact.key);
        self.overrides.insert(fact.key.clone(), fact);
    }

    pub(crate) fn delete_fact(&mut self, key: &str) {
        self.overrides.remove(key);
        if !self.deletions.iter().any(|k| k == key) {
            self.deletions.push(key.to_string());
        }
    }

    pub(crate) fn resolve_facts<'a>(&'a self, base: &'a [KnowledgeFact]) -> Vec<&'a KnowledgeFact> {
        let mut result: Vec<&KnowledgeFact> = base
            .iter()
            .filter(|f| !self.deletions.contains(&f.key))
            .filter(|f| !self.overrides.contains_key(&f.key))
            .collect();

        for fact in self.overrides.values() {
            result.push(fact);
        }

        result
    }

    pub(crate) fn merge_into(self, knowledge: &mut ProjectKnowledge) -> usize {
        let mut merged = 0;
        for (key, fact) in self.overrides {
            if let Some(existing) = knowledge.facts.iter_mut().find(|f| f.key == key) {
                *existing = fact;
            } else {
                knowledge.facts.push(fact);
            }
            merged += 1;
        }
        for key in &self.deletions {
            knowledge.facts.retain(|f| f.key != *key);
        }
        merged
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.overrides.is_empty() && self.deletions.is_empty()
    }

    pub(crate) fn override_count(&self) -> usize {
        self.overrides.len()
    }

    pub(crate) fn deletion_count(&self) -> usize {
        self.deletions.len()
    }

    pub(crate) fn conversation_id(&self) -> &str {
        &self.conversation_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_fact(key: &str, value: &str) -> KnowledgeFact {
        let now = chrono::Utc::now();
        KnowledgeFact {
            category: "test".to_string(),
            key: key.to_string(),
            value: value.to_string(),
            source_session: "test-session".to_string(),
            confidence: 1.0,
            created_at: now,
            last_confirmed: now,
            retrieval_count: 0,
            last_retrieved: None,
            valid_from: None,
            valid_until: None,
            supersedes: None,
            confirmation_count: 0,
            feedback_up: 0,
            feedback_down: 0,
            last_feedback: None,
            privacy: Default::default(),
            sensitivity: Default::default(),
            imported_from: None,
            archetype: Default::default(),
            fidelity: Default::default(),
            revision_count: 0,
        }
    }

    #[test]
    fn branch_overrides_base_fact() {
        let base = vec![test_fact("lang", "rust"), test_fact("db", "postgres")];
        let mut branch = MemoryBranch::new("test-conv");
        branch.override_fact(test_fact("lang", "python"));

        let resolved = branch.resolve_facts(&base);
        assert_eq!(resolved.len(), 2);
        let lang = resolved.iter().find(|f| f.key == "lang").unwrap();
        assert_eq!(lang.value, "python");
    }

    #[test]
    fn branch_deletes_base_fact() {
        let base = vec![test_fact("lang", "rust"), test_fact("db", "postgres")];
        let mut branch = MemoryBranch::new("test-conv");
        branch.delete_fact("db");

        let resolved = branch.resolve_facts(&base);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].key, "lang");
    }

    #[test]
    fn empty_branch_returns_all_base_facts() {
        let base = vec![test_fact("lang", "rust")];
        let branch = MemoryBranch::new("test-conv");
        assert!(branch.is_empty());

        let resolved = branch.resolve_facts(&base);
        assert_eq!(resolved.len(), 1);
    }

    #[test]
    fn merge_applies_overrides_and_deletions() {
        let mut knowledge = ProjectKnowledge::load_or_create("/tmp/lean-ctx-test-memory-branch");
        knowledge.facts.push(test_fact("lang", "rust"));
        knowledge.facts.push(test_fact("db", "postgres"));

        let mut branch = MemoryBranch::new("test-conv");
        branch.override_fact(test_fact("lang", "python"));
        branch.delete_fact("db");

        let merged = branch.merge_into(&mut knowledge);
        assert_eq!(merged, 1);
        assert_eq!(knowledge.facts.len(), 1);
        assert_eq!(knowledge.facts[0].value, "python");
    }
}
