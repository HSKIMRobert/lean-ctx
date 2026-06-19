//! Graph backend selection — which graph engine the provider facade uses.
//!
//! lean-ctx historically carried two graph engines: the mature in-memory
//! `graph_index` (JSON-backed, fully populated in production) and the newer
//! SQLite-backed `PropertyGraph` (scalable, but not yet production-complete —
//! see #682). This flag makes the choice explicit and, crucially, keeps the
//! default on the proven engine so completing the PropertyGraph can never
//! silently flip production onto an unverified backend.

use serde::{Deserialize, Serialize};

use super::Config;

/// Which graph engine [`crate::core::graph_provider::open_best_effort`] selects.
///
/// - `Legacy`: (Default) Always use the in-memory `graph_index`. The
///   PropertyGraph is never consulted — the safe choice while it is being
///   completed and proven equivalent (#682.1–#682.3).
/// - `Auto`: Best-effort — prefer the PropertyGraph when it is fully populated
///   (nodes + edges + file catalog), otherwise fall back to `graph_index`.
/// - `PropertyGraph`: Prefer the PropertyGraph; `graph_index` only as a safety
///   fallback when the PropertyGraph is unavailable.
///
/// Override via the `LEAN_CTX_GRAPH_BACKEND` env var.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GraphBackend {
    #[default]
    Legacy,
    Auto,
    PropertyGraph,
}

impl GraphBackend {
    /// Reads the backend from the `LEAN_CTX_GRAPH_BACKEND` env var, if set to a
    /// recognized value. Accepts a few spellings for ergonomics.
    pub fn from_env() -> Option<Self> {
        std::env::var("LEAN_CTX_GRAPH_BACKEND").ok().and_then(|v| {
            match v.trim().to_lowercase().as_str() {
                "legacy" | "graph-index" | "graph_index" => Some(Self::Legacy),
                "auto" | "best-effort" => Some(Self::Auto),
                "property-graph" | "property_graph" | "propertygraph" | "pg" => {
                    Some(Self::PropertyGraph)
                }
                _ => None,
            }
        })
    }

    /// The effective backend: env override wins over the configured value.
    pub fn effective(config: &Config) -> Self {
        Self::from_env().unwrap_or(config.graph_backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_legacy() {
        assert_eq!(GraphBackend::default(), GraphBackend::Legacy);
    }

    #[test]
    fn serde_roundtrip_kebab() {
        #[derive(Deserialize)]
        struct Wrapper {
            graph_backend: GraphBackend,
        }
        let w: Wrapper = toml::from_str(r#"graph_backend = "property-graph""#).unwrap();
        assert_eq!(w.graph_backend, GraphBackend::PropertyGraph);
    }

    #[test]
    fn effective_falls_back_to_config_without_env() {
        let _lock = crate::core::data_dir::test_env_lock();
        crate::test_env::remove_var("LEAN_CTX_GRAPH_BACKEND");
        let cfg = Config {
            graph_backend: GraphBackend::Auto,
            ..Config::default()
        };
        assert_eq!(GraphBackend::effective(&cfg), GraphBackend::Auto);
    }

    #[test]
    fn env_overrides_config() {
        let _lock = crate::core::data_dir::test_env_lock();
        crate::test_env::set_var("LEAN_CTX_GRAPH_BACKEND", "pg");
        let cfg = Config {
            graph_backend: GraphBackend::Legacy,
            ..Config::default()
        };
        assert_eq!(GraphBackend::effective(&cfg), GraphBackend::PropertyGraph);
        crate::test_env::remove_var("LEAN_CTX_GRAPH_BACKEND");
    }
}
