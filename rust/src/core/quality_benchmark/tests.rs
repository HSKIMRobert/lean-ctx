use super::*;

fn call(name: &str, original: u64, delivered: u64) -> RecordedToolCall {
    RecordedToolCall {
        name: name.into(),
        source: Some("src/lib.rs".into()),
        mode: Some("map".into()),
        original_tokens: original,
        delivered_tokens: delivered,
    }
}

fn arm(success: bool, turns: u64, usage: TokenUsage, calls: Vec<RecordedToolCall>) -> RecordedArm {
    RecordedArm {
        success,
        turns,
        usage,
        tool_calls: calls,
    }
}

fn suite(without: RecordedArm, with: RecordedArm) -> ReplaySuite {
    ReplaySuite {
        kind: REPLAY_KIND.into(),
        sessions: vec![RecordedSession {
            id: "task-1".into(),
            model: "claude-opus-4.5".into(),
            without_compression: without,
            with_compression: with,
        }],
    }
}

#[test]
fn prices_all_four_token_classes_at_real_opus_rates() {
    let usage = TokenUsage {
        new_input_tokens: 1_000_000,
        cache_read_tokens: 1_000_000,
        cache_write_tokens: 1_000_000,
        output_tokens: 1_000_000,
    };
    let report = replay(&suite(
        arm(true, 1, usage, vec![]),
        arm(true, 1, usage, vec![]),
    ))
    .unwrap();
    assert_eq!(report.without.total_cost_usd, 36.75);
}

#[test]
fn cache_breakage_can_make_compression_more_expensive() {
    let baseline = TokenUsage {
        cache_read_tokens: 1_000_000,
        ..TokenUsage::default()
    };
    let compressed = TokenUsage {
        cache_write_tokens: 1_000_000,
        ..TokenUsage::default()
    };
    let report = replay(&suite(
        arm(true, 2, baseline, vec![]),
        arm(true, 2, compressed, vec![]),
    ))
    .unwrap();
    assert_eq!(report.total_savings_usd, -5.75);
}

#[test]
fn output_turn_tax_dominates_cache_read_savings() {
    let baseline = TokenUsage {
        cache_read_tokens: 100_000,
        ..TokenUsage::default()
    };
    let compressed = TokenUsage {
        output_tokens: 10_000,
        ..TokenUsage::default()
    };
    let report = replay(&suite(
        arm(true, 1, baseline, vec![]),
        arm(true, 2, compressed, vec![]),
    ))
    .unwrap();
    assert_eq!(report.without.total_cost_usd, 0.05);
    assert_eq!(report.with.total_cost_usd, 0.25);
    assert_eq!(report.mean_turn_delta, 1.0);
}

#[test]
fn expand_rate_uses_all_tool_calls_and_bounce_tracker() {
    let calls = vec![
        call("ctx_read", 1_000, 100),
        call("ctx_expand", 0, 80),
        call("ctx_search", 500, 200),
    ];
    let report = replay(&suite(
        arm(true, 1, TokenUsage::default(), vec![]),
        arm(true, 1, TokenUsage::default(), calls),
    ))
    .unwrap();
    assert_eq!(report.with.expansions, 1);
    assert_eq!(report.with.tool_calls, 3);
    assert_eq!(report.with.compression_saved_tokens, 1_200);
}

#[test]
fn success_wilson_interval_is_bounded() {
    let report = replay(&suite(
        arm(false, 1, TokenUsage::default(), vec![]),
        arm(true, 1, TokenUsage::default(), vec![]),
    ))
    .unwrap();
    assert!(report.with.success_ci.low >= 0.0);
    assert!(report.with.success_ci.high <= 1.0);
    assert_eq!(report.mean_success_delta, 1.0);
}

#[test]
fn report_is_byte_deterministic() {
    let input = suite(
        arm(true, 2, TokenUsage::default(), vec![]),
        arm(true, 2, TokenUsage::default(), vec![]),
    );
    assert_eq!(
        format_markdown(&replay(&input).unwrap()),
        format_markdown(&replay(&input).unwrap())
    );
}

#[test]
fn validation_rejects_duplicate_session_ids() {
    let mut input = suite(
        arm(true, 1, TokenUsage::default(), vec![]),
        arm(true, 1, TokenUsage::default(), vec![]),
    );
    input.sessions.push(input.sessions[0].clone());
    assert!(
        replay(&input)
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
}

#[test]
fn replay_rejects_estimated_model_pricing() {
    let mut input = suite(
        arm(true, 1, TokenUsage::default(), vec![]),
        arm(true, 1, TokenUsage::default(), vec![]),
    );
    input.sessions[0].model = "future-unknown-model".into();
    assert!(
        replay(&input)
            .unwrap_err()
            .to_string()
            .contains("no exact embedded price")
    );
}

#[test]
fn markdown_contains_confidence_intervals_and_dollar_impact() {
    let report = replay(&suite(
        arm(true, 2, TokenUsage::default(), vec![]),
        arm(true, 2, TokenUsage::default(), vec![]),
    ))
    .unwrap();
    let markdown = format_markdown(&report);
    assert!(markdown.contains("95% confidence intervals"));
    assert!(markdown.contains("Total dollar savings"));
    assert!(!markdown.contains("timestamp"));
}
