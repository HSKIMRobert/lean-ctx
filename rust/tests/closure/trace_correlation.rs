use lean_ctx::core::ocla::types::OclaRequestContext;

fn request_context(trace_id: Option<&str>) -> OclaRequestContext {
    OclaRequestContext::new(
        "request-1".into(),
        "session-1".into(),
        "agent-1".into(),
        "content:fixture".into(),
        None,
        trace_id.map(str::to_owned),
    )
}

#[test]
fn trace_id_generated_when_absent() {
    let trace_id = request_context(None).trace_id;
    assert!(!trace_id.is_empty(), "must generate a trace ID");
    assert!(trace_id.len() > 8, "trace ID must be non-trivial");
}

#[test]
fn trace_id_preserved_when_present() {
    assert_eq!(
        request_context(Some("test-trace-123")).trace_id,
        "test-trace-123"
    );
}

#[test]
fn trace_id_deterministic_format() {
    let ids: Vec<String> = (0..10).map(|_| request_context(None).trace_id).collect();
    let unique: std::collections::HashSet<&String> = ids.iter().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "generated trace IDs must be unique"
    );

    for id in ids {
        let uuid = id.strip_prefix("tr-").unwrap();
        assert_eq!(uuid.len(), 36, "trace ID must contain a UUID: {id}");
        assert!(
            uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
            "trace ID contains invalid chars: {id}"
        );
    }
}

#[test]
#[ignore = "requires public proxy trace helper; OclaRequestContext accepts an explicit empty trace ID"]
fn empty_trace_id_generates_new() {
    // The proxy helper filters an empty x-trace-id header before constructing
    // OclaRequestContext. That helper is currently pub(super).
}

#[test]
fn trace_id_round_trips_through_ocla_wire_context() {
    let context = request_context(Some("trace-request-proxy-mcp"));
    let wire = serde_json::to_string(&context).unwrap();
    let restored: OclaRequestContext = serde_json::from_str(&wire).unwrap();
    assert_eq!(restored.trace_id, "trace-request-proxy-mcp");
    assert_eq!(restored.request_id, "request-1");
    assert_eq!(restored.session_id, "session-1");
}

#[test]
#[ignore = "requires public proxy trace helpers: extract_or_generate_trace_id and inject_trace_id are pub(super)"]
fn trace_id_injected_into_response() {
    // The public integration contract must exercise x-trace-id request extraction
    // and response injection once the proxy helpers are exported from lean_ctx.
}

#[test]
#[ignore = "requires SavingsEvent request_id, session_id, and trace_id fields"]
fn savings_record_contains_trace_id() {
    // The public ledger currently cannot record the OCLA correlation identifiers.
}
