//! Slim default MCP tool surface.

/// The minimal tool set that covers 95% of agent workflows.
/// Every other tool is reachable via ctx_call (always included).
pub const SLIM_CORE_NAMES: &[&str] = &[
    "ctx_read",
    "ctx_shell",
    "shell",
    "ctx_search",
    "ctx_call",
    "ctx_expand",
];

/// Whether the slim-core surface should be used.
/// Enabled by default; disable with LEAN_CTX_FULL_TOOLS=1 or
/// LEAN_CTX_SLIM_CORE=0.
#[must_use]
pub fn slim_core_enabled() -> bool {
    if crate::tool_defs::is_full_mode() {
        return false;
    }
    !std::env::var("LEAN_CTX_SLIM_CORE").is_ok_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
}

/// Filter tools to the slim core set.
#[must_use]
pub fn filter_to_slim_core(tools: Vec<rmcp::model::Tool>) -> Vec<rmcp::model::Tool> {
    tools
        .into_iter()
        .filter(|t| SLIM_CORE_NAMES.contains(&t.name.as_ref()))
        .collect()
}

/// Returns the names of tools that were removed by slim filtering.
/// Used by the summary pool to know which tools to catalog.
#[must_use]
pub fn hidden_by_slim<'a>(all_core: &'a [&'a str]) -> Vec<&'a str> {
    all_core
        .iter()
        .filter(|name| !SLIM_CORE_NAMES.contains(name))
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{SLIM_CORE_NAMES, filter_to_slim_core, hidden_by_slim, slim_core_enabled};

    fn with_clean_slim_env(test: impl FnOnce()) {
        let _guard = crate::core::data_dir::test_env_lock();
        let full_tools = std::env::var_os("LEAN_CTX_FULL_TOOLS");
        let lazy_tools = std::env::var_os("LEAN_CTX_LAZY_TOOLS");
        let slim_core = std::env::var_os("LEAN_CTX_SLIM_CORE");
        crate::test_env::remove_var("LEAN_CTX_FULL_TOOLS");
        crate::test_env::remove_var("LEAN_CTX_LAZY_TOOLS");
        crate::test_env::remove_var("LEAN_CTX_SLIM_CORE");

        test();

        restore_env("LEAN_CTX_FULL_TOOLS", full_tools);
        restore_env("LEAN_CTX_LAZY_TOOLS", lazy_tools);
        restore_env("LEAN_CTX_SLIM_CORE", slim_core);
    }

    fn restore_env(name: &str, value: Option<std::ffi::OsString>) {
        if let Some(value) = value {
            crate::test_env::set_var(name, value);
        } else {
            crate::test_env::remove_var(name);
        }
    }

    #[test]
    fn test_slim_core_enabled_default() {
        with_clean_slim_env(|| assert!(slim_core_enabled()));
    }

    #[test]
    fn test_slim_core_disabled_by_env() {
        with_clean_slim_env(|| {
            crate::test_env::set_var("LEAN_CTX_SLIM_CORE", "0");
            assert!(!slim_core_enabled());
        });
    }

    #[test]
    fn test_slim_core_disabled_when_full() {
        with_clean_slim_env(|| {
            crate::test_env::set_var("LEAN_CTX_FULL_TOOLS", "1");
            assert!(!slim_core_enabled());
        });
    }

    #[test]
    fn test_filter_to_slim_core() {
        let filtered = filter_to_slim_core(crate::server::registry::build_registry().tool_defs());
        assert!(!filtered.is_empty());
        assert!(
            filtered
                .iter()
                .all(|tool| SLIM_CORE_NAMES.contains(&tool.name.as_ref()))
        );
        assert!(filtered.iter().any(|tool| tool.name.as_ref() == "ctx_read"));
        assert!(
            !filtered
                .iter()
                .any(|tool| tool.name.as_ref() == "ctx_graph")
        );
    }

    #[test]
    fn test_hidden_by_slim() {
        let all_core = ["ctx_read", "ctx_tree", "ctx_call", "ctx_session"];
        assert_eq!(hidden_by_slim(&all_core), vec!["ctx_tree", "ctx_session"]);
    }

    #[test]
    fn test_slim_core_always_includes_ctx_call() {
        assert!(SLIM_CORE_NAMES.contains(&"ctx_call"));
    }
}
