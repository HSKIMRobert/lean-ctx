//! Contract tests: verify that generated rules for every IDE are consistent.
//!
//! Checks:
//! - Every rule file contains "ctx_read" or "NEVER.*native Read"
//! - No rule file uses "PREFER" for critical tool mappings
//! - Hybrid agents don't mention "ctx_shell" as preferred (they use lean-ctx -c)
//! - MCP agents don't mention "lean-ctx -c" as preferred (they use ctx_shell)
//! - "MUST" is used instead of "PREFER" for critical mappings

use lean_ctx::rules_inject;

#[test]
fn shared_rules_use_must_not_prefer() {
    let content = rules_inject::rules_shared_content();
    assert!(
        !content.contains("| PREFER"),
        "RULES_SHARED must not use PREFER in table header"
    );
    assert!(
        content.contains("| MUST USE"),
        "RULES_SHARED must use MUST USE in table header"
    );
}

#[test]
fn dedicated_rules_use_must_not_prefer() {
    let content = rules_inject::rules_dedicated_markdown();
    assert!(
        !content.contains("| PREFER"),
        "RULES_DEDICATED must not use PREFER in table header"
    );
    assert!(
        content.contains("| MUST USE"),
        "RULES_DEDICATED must use MUST USE in table header"
    );
}

#[test]
fn all_rules_contain_ctx_read() {
    let shared = rules_inject::rules_shared_content();
    let dedicated = rules_inject::rules_dedicated_markdown();

    assert!(
        shared.contains("ctx_read"),
        "shared rules must mention ctx_read"
    );
    assert!(
        dedicated.contains("ctx_read"),
        "dedicated rules must mention ctx_read"
    );
}

#[test]
fn all_rules_contain_never_reinforcement() {
    let shared = rules_inject::rules_shared_content();
    let dedicated = rules_inject::rules_dedicated_markdown();

    assert!(
        shared.contains("NEVER"),
        "shared rules must contain NEVER reinforcement"
    );
    assert!(
        dedicated.contains("NEVER"),
        "dedicated rules must contain NEVER reinforcement"
    );
}

#[test]
fn canonical_hybrid_no_ctx_shell_as_must_use() {
    let table =
        lean_ctx::core::rules_canonical::tool_table(lean_ctx::core::rules_canonical::Mode::Hybrid);
    for line in table.lines() {
        assert!(
            !line.starts_with("| `ctx_shell"),
            "Hybrid table must not list ctx_shell in MUST USE column (first column)"
        );
    }
}

#[test]
fn canonical_mcp_no_lean_ctx_c_preferred() {
    let table =
        lean_ctx::core::rules_canonical::tool_table(lean_ctx::core::rules_canonical::Mode::Mcp);
    assert!(
        !table.contains("lean-ctx -c"),
        "MCP table must not list lean-ctx -c (should use ctx_shell)"
    );
}

#[test]
fn canonical_both_modes_have_must() {
    for mode in [
        lean_ctx::core::rules_canonical::Mode::Hybrid,
        lean_ctx::core::rules_canonical::Mode::Mcp,
    ] {
        let table = lean_ctx::core::rules_canonical::tool_table(mode);
        assert!(
            table.contains("MUST USE"),
            "canonical table for {mode:?} must use MUST USE"
        );
        assert!(
            !table.contains("| PREFER"),
            "canonical table for {mode:?} must not use PREFER"
        );
    }
}

#[test]
fn canonical_mcp_instructions_contain_must() {
    for mode in [
        lean_ctx::core::rules_canonical::Mode::Hybrid,
        lean_ctx::core::rules_canonical::Mode::Mcp,
    ] {
        let instructions = lean_ctx::core::rules_canonical::mcp_instructions(mode);
        assert!(
            instructions.contains("MUST"),
            "MCP instructions for {mode:?} must contain MUST"
        );
        assert!(
            !instructions.contains("PREFER"),
            "MCP instructions for {mode:?} must not contain PREFER"
        );
    }
}

#[test]
fn cursor_mdc_template_has_must_not_prefer() {
    let mdc = include_str!("../src/templates/lean-ctx.mdc");
    assert!(mdc.contains("MUST USE"), "Cursor MDC must use MUST USE");
    assert!(
        !mdc.contains("| PREFER"),
        "Cursor MDC must not use PREFER in table"
    );
    assert!(
        mdc.contains("NEVER"),
        "Cursor MDC must contain NEVER reinforcement"
    );
}

#[test]
fn hybrid_mdc_template_has_must_not_prefer() {
    let mdc = include_str!("../src/templates/lean-ctx-hybrid.mdc");
    assert!(mdc.contains("MUST USE"), "Hybrid MDC must use MUST USE");
    assert!(
        !mdc.contains("| PREFER"),
        "Hybrid MDC must not use PREFER in table"
    );
}

#[test]
fn no_contradictions_in_hybrid_mdc() {
    let mdc = include_str!("../src/templates/lean-ctx-hybrid.mdc");
    for line in mdc.lines() {
        assert!(
            !line.starts_with("| `ctx_shell"),
            "Hybrid MDC must not list ctx_shell in MUST USE column (first column)"
        );
    }
}
