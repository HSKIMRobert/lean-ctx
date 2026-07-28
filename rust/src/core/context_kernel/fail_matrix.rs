//! Fail-open and fail-closed behavior for Context Kernel subsystems.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::core::ocla::types::FailMode;

/// Categories of subsystems with fail mode assignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubsystemClass {
    Compression,
    Routing,
    PolicyEval,
    TokenAccounting,
    CacheLayer,
    ExternalProvider,
    A2aTransport,
    ResponseShaping,
}

/// A matrix entry mapping a subsystem to its fail behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailMatrixEntry {
    pub subsystem: SubsystemClass,
    pub mode: FailMode,
    pub rationale: String,
}

/// The complete fail matrix with defaults and custom overrides.
pub struct FailMatrix {
    entries: HashMap<SubsystemClass, FailMatrixEntry>,
}

#[derive(Debug, Default, Deserialize)]
struct FailMatrixConfig {
    #[serde(default)]
    entries: Vec<FailMatrixEntry>,
}

impl FailMatrix {
    /// Returns the canonical production fail behavior for every subsystem.
    pub fn production_defaults() -> Self {
        let entries = [
            entry(
                SubsystemClass::Compression,
                FailMode::Open,
                "Degrade to uncompressed output.",
            ),
            entry(
                SubsystemClass::Routing,
                FailMode::Open,
                "Use the default model.",
            ),
            entry(
                SubsystemClass::PolicyEval,
                FailMode::Closed,
                "Deny requests when policy cannot be evaluated.",
            ),
            entry(
                SubsystemClass::TokenAccounting,
                FailMode::Open,
                "Skip accounting without blocking the request.",
            ),
            entry(
                SubsystemClass::CacheLayer,
                FailMode::Open,
                "Bypass the cache.",
            ),
            entry(
                SubsystemClass::ExternalProvider,
                FailMode::Closed,
                "Do not forward when the provider is unreachable.",
            ),
            entry(
                SubsystemClass::A2aTransport,
                FailMode::Open,
                "Process locally.",
            ),
            entry(
                SubsystemClass::ResponseShaping,
                FailMode::Open,
                "Return the unmodified response.",
            ),
        ];

        Self {
            entries: entries
                .into_iter()
                .map(|entry| (entry.subsystem, entry))
                .collect(),
        }
    }

    /// Loads fail-mode overrides from `fail-matrix.toml` and merges them with defaults.
    pub fn from_config() -> Self {
        let mut matrix = Self::production_defaults();
        let Some(contents) = crate::core::paths::config_dir()
            .ok()
            .map(|directory| directory.join("fail-matrix.toml"))
            .and_then(|path| std::fs::read_to_string(path).ok())
        else {
            return matrix;
        };
        let Ok(config) = toml::from_str::<FailMatrixConfig>(&contents) else {
            return matrix;
        };

        matrix.merge_entries(config.entries);
        matrix
    }

    /// Resolves the fail behavior assigned to a subsystem.
    #[must_use]
    pub fn resolve(&self, subsystem: SubsystemClass) -> FailMode {
        self.entries
            .get(&subsystem)
            .map(|entry| entry.mode)
            .expect("production fail matrix includes every subsystem")
    }

    /// Returns whether processing may continue after this subsystem fails.
    #[must_use]
    pub fn should_proceed(&self, subsystem: SubsystemClass) -> bool {
        self.resolve(subsystem) == FailMode::Open
    }

    /// Overrides a subsystem's fail behavior and records the reason for it.
    pub fn override_mode(&mut self, subsystem: SubsystemClass, mode: FailMode, rationale: &str) {
        self.entries.insert(
            subsystem,
            FailMatrixEntry {
                subsystem,
                mode,
                rationale: rationale.to_owned(),
            },
        );
    }

    /// Returns all entries ordered by their snake-case subsystem names.
    #[must_use]
    pub fn report(&self) -> Vec<FailMatrixEntry> {
        let mut entries: Vec<_> = self.entries.values().cloned().collect();
        entries.sort_by_key(|entry| subsystem_name(entry.subsystem));
        entries
    }

    fn merge_entries(&mut self, overrides: impl IntoIterator<Item = FailMatrixEntry>) {
        for entry in overrides {
            self.entries.insert(entry.subsystem, entry);
        }
    }
}

fn entry(subsystem: SubsystemClass, mode: FailMode, rationale: &str) -> FailMatrixEntry {
    FailMatrixEntry {
        subsystem,
        mode,
        rationale: rationale.to_owned(),
    }
}

fn subsystem_name(subsystem: SubsystemClass) -> &'static str {
    match subsystem {
        SubsystemClass::Compression => "compression",
        SubsystemClass::Routing => "routing",
        SubsystemClass::PolicyEval => "policy_eval",
        SubsystemClass::TokenAccounting => "token_accounting",
        SubsystemClass::CacheLayer => "cache_layer",
        SubsystemClass::ExternalProvider => "external_provider",
        SubsystemClass::A2aTransport => "a2a_transport",
        SubsystemClass::ResponseShaping => "response_shaping",
    }
}

#[cfg(test)]
mod tests {
    use super::{FailMatrix, FailMatrixConfig, SubsystemClass};
    use crate::core::ocla::types::FailMode;

    #[test]
    fn production_defaults_assign_correct_fail_modes() {
        let matrix = FailMatrix::production_defaults();

        assert_eq!(matrix.resolve(SubsystemClass::Compression), FailMode::Open);
        assert_eq!(matrix.resolve(SubsystemClass::Routing), FailMode::Open);
        assert_eq!(
            matrix.resolve(SubsystemClass::TokenAccounting),
            FailMode::Open
        );
        assert_eq!(matrix.resolve(SubsystemClass::CacheLayer), FailMode::Open);
        assert_eq!(matrix.resolve(SubsystemClass::A2aTransport), FailMode::Open);
        assert_eq!(
            matrix.resolve(SubsystemClass::ResponseShaping),
            FailMode::Open
        );
    }

    #[test]
    fn policy_eval_and_external_provider_fail_closed_by_default() {
        let matrix = FailMatrix::production_defaults();

        assert_eq!(matrix.resolve(SubsystemClass::PolicyEval), FailMode::Closed);
        assert_eq!(
            matrix.resolve(SubsystemClass::ExternalProvider),
            FailMode::Closed
        );
    }

    #[test]
    fn override_changes_mode_and_rationale() {
        let mut matrix = FailMatrix::production_defaults();
        matrix.override_mode(
            SubsystemClass::Routing,
            FailMode::Closed,
            "Routing must be available for this deployment.",
        );

        let routing = matrix
            .report()
            .into_iter()
            .find(|entry| entry.subsystem == SubsystemClass::Routing)
            .expect("routing entry exists");
        assert_eq!(routing.mode, FailMode::Closed);
        assert_eq!(
            routing.rationale,
            "Routing must be available for this deployment."
        );
    }

    #[test]
    fn should_proceed_reflects_fail_mode() {
        let matrix = FailMatrix::production_defaults();

        assert!(matrix.should_proceed(SubsystemClass::Compression));
        assert!(!matrix.should_proceed(SubsystemClass::PolicyEval));
    }

    #[test]
    fn report_is_sorted_by_subsystem_name() {
        let matrix = FailMatrix::production_defaults();
        let subsystems: Vec<_> = matrix
            .report()
            .into_iter()
            .map(|entry| entry.subsystem)
            .collect();

        assert_eq!(
            subsystems,
            vec![
                SubsystemClass::A2aTransport,
                SubsystemClass::CacheLayer,
                SubsystemClass::Compression,
                SubsystemClass::ExternalProvider,
                SubsystemClass::PolicyEval,
                SubsystemClass::ResponseShaping,
                SubsystemClass::Routing,
                SubsystemClass::TokenAccounting,
            ]
        );
    }

    #[test]
    fn config_merge_overrides_one_entry_and_retains_defaults() {
        let config: FailMatrixConfig = toml::from_str(
            r#"
                [[entries]]
                subsystem = "routing"
                mode = "closed"
                rationale = "Require explicit routing."
            "#,
        )
        .expect("valid fail matrix configuration");
        let mut matrix = FailMatrix::production_defaults();
        matrix.merge_entries(config.entries);

        assert_eq!(matrix.resolve(SubsystemClass::Routing), FailMode::Closed);
        assert_eq!(matrix.resolve(SubsystemClass::Compression), FailMode::Open);
        assert_eq!(matrix.resolve(SubsystemClass::PolicyEval), FailMode::Closed);
    }
}
