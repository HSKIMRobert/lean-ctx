//! Provider initialization — auto-registers all built-in + config-based providers.
//!
//! Called once at startup. Respects the `[providers]` config section:
//! - `providers.enabled = false` → skip all registration
//! - `providers.github.enabled = false` → skip GitHub
//! - `providers.gitlab.enabled = false` → skip GitLab
//!
//! After built-in providers, scans well-known directories for user-defined
//! TOML/JSON provider configs and registers them automatically.

use std::path::Path;
use std::sync::Arc;

use super::config_provider::discovery::discover_configs;
use super::config_provider::ConfigProvider;
use super::github::GitHubProvider;
use super::jira::JiraProvider;
use super::postgres::PostgresProvider;
use super::provider_trait::ContextProvider;
use super::registry::global_registry;

/// Register all built-in providers with the global registry.
/// Respects `[providers]` config for enabling/disabling individual providers.
/// Safe to call multiple times (idempotent — overwrites existing entries).
pub fn init_builtin_providers() {
    init_with_project_root(None);
}

/// Register all providers, including config-based ones scoped to `project_root`.
pub fn init_with_project_root(project_root: Option<&Path>) {
    let cfg = crate::core::config::Config::load();

    if !cfg.providers.enabled {
        tracing::debug!("[providers] subsystem disabled via config");
        return;
    }

    let registry = global_registry();

    // --- Built-in providers ---
    if cfg.providers.github.enabled {
        registry.register(Arc::new(GitHubProvider::new()));
    }

    registry.register(Arc::new(JiraProvider::new()));
    registry.register(Arc::new(PostgresProvider::new()));

    // --- Config-based providers (user-defined) ---
    let discovered = discover_configs(project_root);
    let mut config_count = 0;
    for entry in discovered {
        match ConfigProvider::from_config(entry.config) {
            Ok(provider) => {
                tracing::info!(
                    "[providers] registered config provider '{}' from {}",
                    provider.id(),
                    entry.source_path.display()
                );
                registry.register(Arc::new(provider));
                config_count += 1;
            }
            Err(e) => {
                tracing::warn!("[providers] skipping {}: {e}", entry.source_path.display());
            }
        }
    }

    tracing::debug!(
        "[providers] initialized {} provider(s) ({} config-based), {} available",
        registry.provider_count(),
        config_count,
        registry.available_provider_ids().len(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_registers_github_when_enabled() {
        init_builtin_providers();
        let reg = global_registry();
        assert!(reg.get("github").is_some());
    }
}
