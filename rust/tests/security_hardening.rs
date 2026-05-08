/// Security hardening tests — validates all Critical and High fixes from the
/// bank-readiness audit (2026-05-08).

// ---------------------------------------------------------------------------
// C1 — Dashboard: token not leaked without valid ?token= query
// ---------------------------------------------------------------------------
#[test]
fn dashboard_route_response_omits_token_without_valid_query() {
    let src = include_str!("../src/dashboard/routes/mod.rs");

    assert!(
        src.contains("is_some_and(|q| super::constant_time_eq(q.as_bytes(), expected.as_bytes()))"),
        "C1: dashboard must use constant_time_eq via is_some_and for query token validation"
    );
    assert!(
        src.contains("if valid_query"),
        "C1: token embedding must be gated on valid_query"
    );
}

#[test]
fn dashboard_check_auth_uses_constant_time_eq() {
    let src = include_str!("../src/dashboard/mod.rs");
    assert!(
        src.contains("constant_time_eq(token.trim().as_bytes(), expected_token.as_bytes())"),
        "C1: check_auth must use constant_time_eq, not plain =="
    );
}

#[test]
fn dashboard_metrics_requires_auth() {
    let src = include_str!("../src/dashboard/mod.rs");
    assert!(
        src.contains(r#"path == "/metrics""#),
        "C1: /metrics must be in the requires_auth path"
    );
}

// ---------------------------------------------------------------------------
// C2 — Team server: workspace enforced from TeamRequestContext
// ---------------------------------------------------------------------------
#[test]
fn context_views_use_resolve_workspace() {
    let src = include_str!("../src/http_server/context_views.rs");

    assert!(
        src.contains("fn resolve_workspace"),
        "C2: context_views must have a resolve_workspace helper"
    );
    let search_fn = src
        .find("v1_events_search")
        .expect("v1_events_search missing");
    let search_body = &src[search_fn..search_fn + 600];
    assert!(
        search_body.contains("resolve_workspace("),
        "C2: v1_events_search must call resolve_workspace"
    );
}

#[test]
fn lineage_filters_by_workspace() {
    let src = include_str!("../src/core/context_os/context_bus.rs");

    let lineage_fn = src.find("fn lineage(").expect("lineage fn missing");
    let lineage_sig = &src[lineage_fn..lineage_fn + 200];
    assert!(
        lineage_sig.contains("workspace_id: &str"),
        "C2: lineage() must take workspace_id parameter"
    );

    let lineage_body = &src[lineage_fn..lineage_fn + 800];
    assert!(
        lineage_body.contains("AND workspace_id = ?2"),
        "C2: lineage SQL must filter by workspace_id"
    );
}

// ---------------------------------------------------------------------------
// H1 — Shell CWD jail enforcement
// ---------------------------------------------------------------------------
#[test]
fn effective_cwd_calls_jail() {
    let src = include_str!("../src/core/session.rs");

    let ecwd_fn = src
        .find("fn effective_cwd(")
        .expect("effective_cwd missing");
    let ecwd_body = &src[ecwd_fn..ecwd_fn + 500];
    assert!(
        ecwd_body.contains("jail_cwd(cwd, root)"),
        "H1: effective_cwd must call jail_cwd for explicit cwd"
    );
}

#[test]
fn update_shell_cwd_calls_jail_path() {
    let src = include_str!("../src/core/session.rs");

    let uscwd_fn = src
        .find("fn update_shell_cwd(")
        .expect("update_shell_cwd missing");
    let uscwd_body = &src[uscwd_fn..uscwd_fn + 600];
    assert!(
        uscwd_body.contains("jail_path("),
        "H1: update_shell_cwd must jail_path check before storing"
    );
}

// ---------------------------------------------------------------------------
// H2 — MCP ctx_read path has secret check
// ---------------------------------------------------------------------------
#[test]
fn resolve_path_includes_secret_check() {
    let src = include_str!("../src/tools/mod.rs");

    let resolve_fn = src.find("fn resolve_path(").expect("resolve_path missing");
    let resolve_body = &src[resolve_fn..];
    let end = resolve_body
        .find("fn resolve_path_or_passthrough")
        .unwrap_or(resolve_body.len());
    let resolve_body = &resolve_body[..end];
    assert!(
        resolve_body.contains("check_secret_path_for_tool"),
        "H2: resolve_path must call check_secret_path_for_tool"
    );
}

// ---------------------------------------------------------------------------
// H3 — REST event responses are redacted
// ---------------------------------------------------------------------------
#[test]
fn event_search_applies_redaction() {
    let src = include_str!("../src/http_server/context_views.rs");

    let search_fn = src
        .find("v1_events_search")
        .expect("v1_events_search missing");
    let search_body = &src[search_fn..search_fn + 800];
    assert!(
        search_body.contains("redact_event_payload("),
        "H3: v1_events_search must redact event payloads"
    );
}

#[test]
fn event_lineage_applies_redaction() {
    let src = include_str!("../src/http_server/context_views.rs");

    let lineage_fn = src
        .find("v1_event_lineage")
        .expect("v1_event_lineage missing");
    let lineage_body = &src[lineage_fn..lineage_fn + 800];
    assert!(
        lineage_body.contains("redact_event_payload("),
        "H3: v1_event_lineage must redact event payloads"
    );
}

// ---------------------------------------------------------------------------
// H4 — JSON-RPC batch scope bypass prevention
// ---------------------------------------------------------------------------
#[test]
fn team_auth_rejects_batch_requests() {
    let src = include_str!("../src/http_server/team.rs");

    assert!(
        src.contains("batch_requests_not_supported"),
        "H4: team auth must reject JSON-RPC batch (array) requests"
    );
    assert!(
        src.contains("let mut allow = false;"),
        "H4: team auth must default allow to false"
    );
}

// ---------------------------------------------------------------------------
// H5 — npm postinstall SHA256 verification
// ---------------------------------------------------------------------------
#[test]
fn postinstall_has_sha256_verification() {
    let src = include_str!("../../packages/lean-ctx-bin/postinstall.js");

    assert!(
        src.contains("createHash(\"sha256\")") || src.contains("createHash('sha256')"),
        "H5: postinstall.js must compute SHA256 hash"
    );
    assert!(
        src.contains("SHA256SUMS"),
        "H5: postinstall.js must download SHA256SUMS for verification"
    );
    assert!(
        src.contains("SHA256 mismatch"),
        "H5: postinstall.js must abort on hash mismatch"
    );
}

// ---------------------------------------------------------------------------
// H6 — Pipeline archive redaction
// ---------------------------------------------------------------------------
#[test]
fn pipeline_archive_uses_redacted_output() {
    let src = include_str!("../src/server/pipeline_stages.rs");

    assert!(
        src.contains("redact_text_if_enabled(result_text)"),
        "H6: shell archive must use redact_text_if_enabled before storing"
    );
}

// ---------------------------------------------------------------------------
// M2 — ReDoS guard in ctx_search
// ---------------------------------------------------------------------------
#[test]
fn ctx_search_has_pattern_length_limit() {
    let src = include_str!("../src/tools/ctx_search.rs");

    assert!(
        src.contains("MAX_PATTERN_LEN"),
        "M2: ctx_search must limit pattern length"
    );
    assert!(
        src.contains(".size_limit("),
        "M2: ctx_search must set regex size_limit"
    );
    assert!(
        src.contains("RegexBuilder::new("),
        "M2: ctx_search must use RegexBuilder (not Regex::new) for size limits"
    );
}

// ---------------------------------------------------------------------------
// M3 — MCP stdio max_length
// ---------------------------------------------------------------------------
#[test]
fn mcp_stdio_has_bounded_max_length() {
    let src = include_str!("../src/mcp_stdio.rs");

    assert!(
        !src.contains("max_length: usize::MAX"),
        "M3: MCP stdio must NOT use usize::MAX for max_length"
    );
    assert!(
        src.contains("32 * 1024 * 1024"),
        "M3: MCP stdio max_length must be 32 MiB"
    );
}

// ---------------------------------------------------------------------------
// M5 — UDS socket permissions
// ---------------------------------------------------------------------------
#[test]
fn uds_socket_sets_permissions() {
    let src = include_str!("../src/http_server/mod.rs");

    assert!(
        src.contains("PermissionsExt"),
        "M5: http_server must import PermissionsExt"
    );
    assert!(
        src.contains("0o600"),
        "M5: http_server must set socket permissions to 0o600"
    );
}

// ---------------------------------------------------------------------------
// L1 — Error responses sanitized
// ---------------------------------------------------------------------------
#[test]
fn http_server_sanitizes_error_responses() {
    let src = include_str!("../src/http_server/mod.rs");

    let v1_fn = src.find("v1_tool_call").expect("v1_tool_call missing");
    let v1_body = &src[v1_fn..v1_fn + 600];
    assert!(
        !v1_body.contains("e.to_string()"),
        "L1: v1_tool_call must not return internal error details"
    );
}
