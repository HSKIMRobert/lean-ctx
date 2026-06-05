//! Pure tool-visibility policy for the MCP `tools/list` response.
//!
//! Extracted from the (async, server-bound) `list_tools` handler so the policy
//! is unit-testable in isolation. The handler resolves the candidate set
//! (lazy-core vs profile-authoritative vs full registry) and the per-call gates
//! (role, workflow), then defers to these helpers for the stable rules:
//!   * Internal/meta tools are never advertised.
//!   * The active profile, `disabled_tools`, and the Zed `ctx_edit` quirk filter
//!     the candidates.
//!   * The universal invoker (`ctx_call`) is force-advertised in non-full mode so
//!     tools hidden by lazy/profile filtering stay reachable.

use super::dynamic_tools::{categorize_tool, ToolCategory};
use crate::core::tool_profiles::ToolProfile;

/// The universal invoker tool name. A static-list MCP client can call any
/// registered tool through it, even when that tool isn't advertised.
pub const INVOKER: &str = "ctx_call";

/// Decides whether a tool name should appear in `tools/list`.
///
/// `role_allows` is supplied by the caller (it depends on the active role, which
/// is resolved outside this pure function). Internal tools are hidden
/// unconditionally — they're invoked automatically or via [`INVOKER`].
#[must_use]
pub fn is_tool_visible(
    name: &str,
    profile: &ToolProfile,
    disabled: &[String],
    is_zed: bool,
    role_allows: bool,
) -> bool {
    if categorize_tool(name) == ToolCategory::Internal {
        return false;
    }
    if !profile.is_tool_enabled(name) {
        return false;
    }
    if disabled.iter().any(|d| d == name) {
        return false;
    }
    if is_zed && name == "ctx_edit" {
        return false;
    }
    role_allows
}

/// Whether [`INVOKER`] must be force-added to the advertised set.
///
/// True only in non-full mode when it isn't already present, the role permits
/// it, and it isn't explicitly disabled. In full mode every tool is already
/// listed, so no gateway is needed.
#[must_use]
pub fn needs_invoker(
    full_mode: bool,
    already_present: bool,
    invoker_role_allowed: bool,
    disabled: &[String],
) -> bool {
    !full_mode && !already_present && invoker_role_allowed && !disabled.iter().any(|d| d == INVOKER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_tools_never_visible_even_in_power() {
        // Power enables everything, but Internal/meta tools must still be hidden.
        let p = ToolProfile::Power;
        assert!(!is_tool_visible("ctx_metrics", &p, &[], false, true));
        assert!(!is_tool_visible("ctx_cost", &p, &[], false, true));
        assert!(!is_tool_visible("ctx_discover_tools", &p, &[], false, true));
    }

    #[test]
    fn core_tool_visible_under_power() {
        assert!(is_tool_visible(
            "ctx_read",
            &ToolProfile::Power,
            &[],
            false,
            true
        ));
    }

    #[test]
    fn standard_exposes_its_advertised_tools() {
        // These are in STANDARD_TOOLS but were dropped by the old
        // `core ∩ standard` intersection. Profile-authoritative resolution must
        // surface them.
        let p = ToolProfile::Standard;
        assert!(is_tool_visible("ctx_architecture", &p, &[], false, true));
        assert!(is_tool_visible("ctx_semantic_search", &p, &[], false, true));
        assert!(is_tool_visible("ctx_callgraph", &p, &[], false, true));
    }

    #[test]
    fn minimal_hides_non_minimal_tools() {
        let p = ToolProfile::Minimal;
        assert!(is_tool_visible("ctx_read", &p, &[], false, true));
        assert!(!is_tool_visible("ctx_architecture", &p, &[], false, true));
    }

    #[test]
    fn disabled_list_filters() {
        let disabled = vec!["ctx_read".to_string()];
        assert!(!is_tool_visible(
            "ctx_read",
            &ToolProfile::Power,
            &disabled,
            false,
            true
        ));
    }

    #[test]
    fn zed_hides_ctx_edit_only() {
        let p = ToolProfile::Power;
        assert!(!is_tool_visible("ctx_edit", &p, &[], true, true));
        assert!(is_tool_visible("ctx_read", &p, &[], true, true));
    }

    #[test]
    fn role_block_hides_tool() {
        assert!(!is_tool_visible(
            "ctx_read",
            &ToolProfile::Power,
            &[],
            false,
            false
        ));
    }

    #[test]
    fn invoker_added_when_missing_in_lazy_mode() {
        assert!(needs_invoker(false, false, true, &[]));
    }

    #[test]
    fn invoker_not_added_in_full_mode() {
        assert!(!needs_invoker(true, false, true, &[]));
    }

    #[test]
    fn invoker_not_duplicated_when_present() {
        assert!(!needs_invoker(false, true, true, &[]));
    }

    #[test]
    fn invoker_respects_role_and_disabled() {
        assert!(!needs_invoker(false, false, false, &[]));
        assert!(!needs_invoker(
            false,
            false,
            true,
            &["ctx_call".to_string()]
        ));
    }
}
