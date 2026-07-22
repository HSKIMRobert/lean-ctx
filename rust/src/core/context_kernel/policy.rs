//! Context policy filtering for the Context Kernel.

use super::types::{ContextObjectV1, SensitivityLevel};

/// Restrictions applied to candidates before kernel selection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextPolicy {
    pub max_sensitivity: SensitivityLevel,
    pub allowed_sources: Option<Vec<String>>,
    pub blocked_sources: Vec<String>,
    pub budget_cap_tokens: Option<usize>,
    pub retention_days: Option<u32>,
}

/// Applies a [`ContextPolicy`] to context candidates.
pub struct PolicyFilter {
    policy: ContextPolicy,
}

impl PolicyFilter {
    /// Creates a filter backed by the supplied policy.
    pub fn new(policy: ContextPolicy) -> Self {
        Self { policy }
    }

    /// Loads the kernel policy from the lean-ctx configuration directory.
    ///
    /// Missing and invalid configuration gracefully falls back to the default
    /// policy so candidate retrieval remains available.
    pub fn from_config(project_root: &str) -> Self {
        let _ = project_root;
        let policy = crate::core::paths::config_dir()
            .ok()
            .map(|directory| directory.join("kernel-policy.toml"))
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|contents| toml::from_str::<ContextPolicy>(&contents).ok())
            .unwrap_or_else(Self::default_policy);

        Self::new(policy)
    }

    /// Returns the permissive default for ordinary internal context.
    pub fn default_policy() -> ContextPolicy {
        ContextPolicy {
            max_sensitivity: SensitivityLevel::Internal,
            allowed_sources: None,
            blocked_sources: Vec::new(),
            budget_cap_tokens: None,
            retention_days: None,
        }
    }

    /// Filters candidates and applies the optional prefix token budget.
    pub fn apply(&self, candidates: Vec<ContextObjectV1>) -> Vec<ContextObjectV1> {
        let allowed: Vec<ContextObjectV1> = candidates
            .into_iter()
            .filter(|candidate| self.is_allowed(candidate))
            .collect();

        let Some(cap) = self.policy.budget_cap_tokens else {
            return allowed;
        };

        let mut used: usize = 0;
        allowed
            .into_iter()
            .take_while(|candidate| {
                if candidate.token_estimate > cap.saturating_sub(used) {
                    return false;
                }
                used = used.saturating_add(candidate.token_estimate);
                true
            })
            .collect()
    }

    /// Returns whether a candidate satisfies sensitivity and source rules.
    pub fn is_allowed(&self, candidate: &ContextObjectV1) -> bool {
        if sensitivity_rank(&candidate.sensitivity)
            > sensitivity_rank(&self.policy.max_sensitivity)
        {
            return false;
        }

        if self
            .policy
            .allowed_sources
            .as_ref()
            .is_some_and(|sources| !sources.contains(&candidate.source))
        {
            return false;
        }

        !self.policy.blocked_sources.contains(&candidate.source)
    }
}

fn sensitivity_rank(level: &SensitivityLevel) -> u8 {
    match level {
        SensitivityLevel::Public => 0,
        SensitivityLevel::Internal => 1,
        SensitivityLevel::Confidential => 2,
        SensitivityLevel::Restricted => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::{ContextPolicy, PolicyFilter};
    use crate::core::context_kernel::types::{ContextObjectV1, SensitivityLevel};

    fn candidate(source: &str, sensitivity: SensitivityLevel, tokens: usize) -> ContextObjectV1 {
        ContextObjectV1 {
            source: source.to_owned(),
            sensitivity,
            token_estimate: tokens,
            ..ContextObjectV1::default()
        }
    }

    fn policy(max_sensitivity: SensitivityLevel) -> ContextPolicy {
        ContextPolicy {
            max_sensitivity,
            allowed_sources: None,
            blocked_sources: Vec::new(),
            budget_cap_tokens: None,
            retention_days: None,
        }
    }

    #[test]
    fn sensitivity_filter_removes_restricted() {
        let filter = PolicyFilter::new(policy(SensitivityLevel::Internal));
        let candidates: Vec<ContextObjectV1> = vec![
            candidate("public", SensitivityLevel::Public, 10),
            candidate("restricted", SensitivityLevel::Restricted, 10),
        ];

        let filtered = filter.apply(candidates);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source, "public");
    }

    #[test]
    fn allowed_sources_filters_correctly() {
        let mut context_policy = policy(SensitivityLevel::Internal);
        context_policy.allowed_sources = Some(vec!["knowledge".to_owned()]);
        let filter = PolicyFilter::new(context_policy);
        let candidates: Vec<ContextObjectV1> = vec![
            candidate("knowledge", SensitivityLevel::Internal, 10),
            candidate("file", SensitivityLevel::Internal, 10),
        ];

        let filtered = filter.apply(candidates);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source, "knowledge");
    }

    #[test]
    fn blocked_sources_removed() {
        let mut context_policy = policy(SensitivityLevel::Internal);
        context_policy.blocked_sources = vec!["episodic".to_owned()];
        let filter = PolicyFilter::new(context_policy);
        let candidates: Vec<ContextObjectV1> = vec![
            candidate("episodic", SensitivityLevel::Internal, 10),
            candidate("file", SensitivityLevel::Internal, 10),
        ];

        let filtered = filter.apply(candidates);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source, "file");
    }

    #[test]
    fn budget_cap_truncates() {
        let mut context_policy = policy(SensitivityLevel::Internal);
        context_policy.budget_cap_tokens = Some(250);
        let filter = PolicyFilter::new(context_policy);
        let candidates: Vec<ContextObjectV1> = vec![
            candidate("first", SensitivityLevel::Internal, 100),
            candidate("second", SensitivityLevel::Internal, 150),
            candidate("third", SensitivityLevel::Internal, 1),
        ];

        let filtered = filter.apply(candidates);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[1].source, "second");
    }

    #[test]
    fn blocked_source_overrides_allowed_source() {
        let mut context_policy = policy(SensitivityLevel::Internal);
        context_policy.allowed_sources = Some(vec!["knowledge".to_owned()]);
        context_policy.blocked_sources = vec!["knowledge".to_owned()];
        let filter = PolicyFilter::new(context_policy);

        assert!(!filter.is_allowed(&candidate(
            "knowledge",
            SensitivityLevel::Internal,
            10,
        )));
    }
}
