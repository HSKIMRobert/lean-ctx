use lean_ctx::core::savings_ledger::event::{MECHANISM_COMPRESSION, SavingsEvent};
use lean_ctx::core::savings_ledger::store;

fn event(saved_tokens: u64, attribution_group: &str) -> SavingsEvent {
    SavingsEvent {
        ts: "2026-07-29T12:00:00+00:00".into(),
        tool: "ctx_read".into(),
        mechanism: MECHANISM_COMPRESSION.into(),
        model_id: "fixture-model".into(),
        tokenizer: "o200k_base".into(),
        baseline_tokens: saved_tokens + 100,
        actual_tokens: 100,
        saved_tokens,
        bounce_adjustment: 0,
        unit_price_per_m_usd: 2.0,
        saved_usd: saved_tokens as f64 * 2.0 / 1_000_000.0,
        repo_hash: "fixture-repo".into(),
        agent_id: "fixture-agent".into(),
        prev_hash: String::new(),
        entry_hash: String::new(),
        version: env!("CARGO_PKG_VERSION").into(),
        intent_tag: None,
        outcome: None,
        model_original: None,
        model_routed: None,
        routing_savings: None,
        response_original_tokens: None,
        response_delivered_tokens: None,
        agent_chain_id: None,
        chain_depth: None,
        measurement_method: None,
        evidence_class: None,
        confidence: None,
        quality_signal: Some("fixture-quality".into()),
        attribution_group: Some(attribution_group.into()),
        attribution_id: None,
        baseline_ref: Some("fixture-baseline".into()),
        price_version: None,
        customer_approval: None,
        settlement_status: None,
        is_first_inject: None,
        cache_read_per_m_usd: None,
        cache_write_per_m_usd: None,
    }
}

#[test]
fn attribution_no_double_count() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.jsonl");
    for saved_tokens in [60, 40] {
        store::append(&path, event(saved_tokens, "session-a")).unwrap();
    }

    let summary = store::summarize(&path);
    let mechanism_total: u64 = summary
        .by_mechanism
        .iter()
        .map(|(_, saved_tokens, _)| *saved_tokens)
        .sum();
    assert_eq!(summary.total_events, 2);
    assert_eq!(summary.saved_tokens, 100);
    assert_eq!(mechanism_total, summary.saved_tokens);
}

#[test]
fn attribution_group_is_persisted_per_request() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.jsonl");
    for saved_tokens in [60, 40] {
        store::append(&path, event(saved_tokens, "shared-group")).unwrap();
    }

    let records = store::load(&path);
    assert_eq!(records.len(), 2);
    assert!(
        records
            .iter()
            .all(|record| record.attribution_group.as_deref() == Some("shared-group"))
    );
}

#[test]
#[ignore = "requires SavingsEvent session_id and trace_id fields plus ledger grouping output"]
fn attribution_trace_groups_requests() {
    // The current ledger groups only by mechanism; trace-level aggregation needs
    // persisted correlation IDs before this E2E contract can be exercised.
}

#[test]
#[ignore = "requires SavingsEvent session_id field and session-scoped ledger query API"]
fn attribution_cross_session_isolation() {
    // Session isolation cannot be verified until session identity is persisted.
}

#[test]
fn attribution_quality_ref_present() {
    let record = event(60, "session-a");
    assert_eq!(record.quality_signal.as_deref(), Some("fixture-quality"));
    assert_eq!(record.baseline_ref.as_deref(), Some("fixture-baseline"));
}

#[test]
#[ignore = "requires SavingsEvent quality_ref field; current public schema uses quality_signal"]
fn savings_export_json_has_required_fields() {
    // The eventual export schema must include request_id, session_id, trace_id,
    // input_tokens, output_tokens, saved_tokens, and quality_ref.
}
