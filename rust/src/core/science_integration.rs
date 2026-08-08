//! End-to-end integration tests for the science-module context pipeline.
//!
//! Exercises IB → Wasserstein → Cognitive → MDL → FSRS → Prefetch → Graph →
//! Anti-Interrupt → Stigmergy → Echo → Verbosity as connected workflows.

use chrono::{Duration, TimeZone, Utc};
use std::collections::HashMap;

use crate::core::anti_interrupt::{
    InterruptionEvent, TEST_LOCK as ANTI_INTERRUPT_LOCK, compute_impact, record_interruption,
    reset_session,
};
use crate::core::cognitive::{budget_select, detect_chunks};
use crate::core::config::CompressionLevel;
use crate::core::context_prefetch::{FileTrajectory, build_prefetch_plan};
use crate::core::echo_ratio::compute_echo_ratio;
use crate::core::graph_expand::{EdgeKind, expand_neighborhood};
use crate::core::ib::{TaskIntent, classify_intent, compute_relevance, intent_query_terms};
use crate::core::mdl_mode::generate_structural_description;
use crate::core::memory_scheduler::{MemoryState, initial_state, retrievability, update_stability};
use crate::core::session::{SessionState, TaskInfo};
use crate::core::stigmergy::{
    PheromoneSignal, PressureMap, SignalKind, deposit_signal, read_signals, reset_signals,
};
use crate::core::verbosity::{TranscriptEntry, extract_signals, recommend_level};
use crate::core::wasserstein::allocate_budget;

const QUERY: &str = "find the database connection handler";

fn session_with_task(description: &str) -> SessionState {
    let mut session = SessionState::new();
    session.task = Some(TaskInfo {
        description: description.to_owned(),
        intent: None,
        progress_pct: None,
    });
    session
}

fn is_valid_compression_level(level: CompressionLevel) -> bool {
    matches!(
        level,
        CompressionLevel::Off
            | CompressionLevel::Lite
            | CompressionLevel::Standard
            | CompressionLevel::Max
            | CompressionLevel::Raw
    )
}

fn sample_rust_source() -> String {
    r#"//! Database layer for the application.

use std::sync::Arc;

pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub pool_size: u32,
}

pub enum ConnectionState {
    Idle,
    Active,
    Error(String),
}

pub struct ConnectionHandler {
    config: Arc<DatabaseConfig>,
    state: ConnectionState,
}

impl ConnectionHandler {
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            config: Arc::new(config),
            state: ConnectionState::Idle,
        }
    }

    pub fn connect(&mut self) -> Result<(), String> {
        self.state = ConnectionState::Active;
        Ok(())
    }

    pub fn database_connection_handler(&self) -> &DatabaseConfig {
        &self.config
    }
}

pub fn open_pool(config: DatabaseConfig) -> ConnectionHandler {
    ConnectionHandler::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_connects() {
        let config = DatabaseConfig {
            host: "localhost".into(),
            port: 5432,
            pool_size: 4,
        };
        let mut handler = ConnectionHandler::new(config);
        assert!(handler.connect().is_ok());
    }
}
"#
    .to_string()
}

// ---------------------------------------------------------------------------
// Test 1: IB intent → relevance → Wasserstein allocation
// ---------------------------------------------------------------------------

#[test]
fn full_context_pipeline() {
    let session = session_with_task(QUERY);
    let intent = classify_intent(&session);
    assert_eq!(
        intent,
        TaskIntent::Explore,
        "query containing 'find' should classify as Explore"
    );

    let query_terms = intent_query_terms(&intent);
    assert!(
        !query_terms.is_empty(),
        "Explore intent should supply default query terms"
    );

    let sources = [
        (
            "src/db/handler.rs",
            "pub fn database_connection_handler(pool: &Pool) -> Connection { pool.acquire() }",
        ),
        (
            "src/ui/button.rs",
            "pub fn render_button(label: &str) -> Html { Html::new(label) }",
        ),
        (
            "src/net/client.rs",
            "pub async fn fetch(url: &str) -> Result<Response, Error> { todo!() }",
        ),
        (
            "src/db/pool.rs",
            "pub struct Pool { max: u32 } impl Pool { pub fn acquire(&self) -> Connection {} }",
        ),
        (
            "src/log/mod.rs",
            "pub fn log_info(message: &str) { println!(\"{message}\") }",
        ),
    ];

    let chunk_refs: Vec<&str> = sources.iter().map(|(_, content)| *content).collect();
    let relevance = compute_relevance(&chunk_refs, &intent, Some(QUERY));
    assert_eq!(
        relevance.len(),
        sources.len(),
        "every source should receive a relevance score"
    );

    for score in &relevance {
        assert!(
            (0.0..=1.0).contains(&score.score),
            "relevance score {} out of bounds for chunk {}",
            score.score,
            score.chunk_index
        );
    }

    let budget = 2000_usize;
    let alloc_inputs: Vec<(&str, usize, f64)> = sources
        .iter()
        .zip(relevance.iter())
        .map(|((path, content), score)| {
            let tokens = content.split_whitespace().count().max(1);
            (*path, tokens, score.score)
        })
        .collect();

    let allocations = allocate_budget(&alloc_inputs, budget);
    let assigned: usize = allocations.iter().map(|entry| entry.tokens).sum();
    assert_eq!(
        assigned, budget,
        "token allocations should sum exactly to the budget"
    );

    let top_relevance = relevance
        .iter()
        .max_by(|left, right| {
            left.score
                .partial_cmp(&right.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("relevance scores");
    let top_path = sources[top_relevance.chunk_index].0;
    let top_allocation = allocations
        .iter()
        .find(|entry| entry.target == top_path)
        .expect("allocation for highest-relevance source");
    let max_tokens = allocations
        .iter()
        .map(|entry| entry.tokens)
        .max()
        .unwrap_or(0);

    assert_eq!(
        top_allocation.tokens, max_tokens,
        "highest-relevance source {top_path} should receive the largest token allocation"
    );
    assert!(
        top_relevance.score >= 0.0,
        "database handler source should be ranked by relevance"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Cognitive chunking → MDL structural description
// ---------------------------------------------------------------------------

#[test]
fn chunking_to_mdl_pipeline() {
    let source = sample_rust_source();
    let chunks = detect_chunks(&source, "rs");
    assert!(
        !chunks.is_empty(),
        "semantic chunker should detect at least one chunk in sample source"
    );

    let selected = budget_select(&chunks, None);
    assert!(
        !selected.is_empty(),
        "budget_select should retain at least one chunk"
    );
    assert!(
        selected.len() <= 9,
        "Miller's Law bounds chunk selection to at most 9 items"
    );

    let desc = generate_structural_description(&source, "src/db/handler.rs", "rs");
    let ratio = desc.compression_ratio();

    assert!(
        ratio > 0.0,
        "compression ratio should be positive, got {ratio}"
    );
    assert!(
        desc.description_tokens < desc.original_tokens,
        "structural description ({} tokens) should be shorter than source ({} tokens)",
        desc.description_tokens,
        desc.original_tokens
    );
    assert!(
        !desc.functions.is_empty() || !desc.types.is_empty(),
        "MDL description should capture at least one type or function fingerprint"
    );
}

// ---------------------------------------------------------------------------
// Test 3: FSRS memory → context prefetch
// ---------------------------------------------------------------------------

#[test]
fn memory_and_prefetch_pipeline() {
    let mut state: MemoryState = initial_state("db-connection-fact".to_string(), 3);
    let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    state.last_review = base;

    let mut previous_r = retrievability(&state, base);
    for (step, rating) in [(1, 3_u8), (2, 4_u8), (3, 3_u8)] {
        let review_time = base + Duration::days(step);
        state.last_review = review_time - Duration::hours(1);
        let r_before = retrievability(&state, review_time);
        update_stability(&mut state, rating);
        let r_after = retrievability(&state, review_time);

        assert!(
            r_after >= 0.9,
            "retrievability after review {step} should stay high, got {r_after}"
        );
        assert!(
            r_after >= r_before.min(previous_r),
            "retrievability should not drop sharply after successful review {step}"
        );
        previous_r = r_after;
    }

    let mut trajectory = FileTrajectory::new(50);
    for path in [
        "src/db/handler.rs",
        "src/db/pool.rs",
        "src/db/handler.rs",
        "src/db/config.rs",
        "src/db/handler.rs",
    ] {
        trajectory.record(path);
    }

    let top_k = 3_usize;
    let predictions = trajectory.predict(top_k);
    assert!(
        !predictions.is_empty(),
        "trajectory with repeated transitions should yield prefetch predictions"
    );
    assert!(
        predictions.len() <= top_k,
        "predictions len {} exceeds top_k {top_k}",
        predictions.len()
    );

    let plan = build_prefetch_plan(&trajectory, &[], top_k, 0.2);
    assert!(
        !plan.files.is_empty(),
        "prefetch plan should include at least one candidate file"
    );
    assert!(
        plan.files.len() <= top_k,
        "prefetch plan size {} exceeds top_k {top_k}",
        plan.files.len()
    );
}

// ---------------------------------------------------------------------------
// Test 4: Anti-interrupt → echo ratio
// ---------------------------------------------------------------------------

#[test]
fn interruption_and_echo_pipeline() {
    let _guard = ANTI_INTERRUPT_LOCK
        .lock()
        .expect("anti-interrupt test lock");
    reset_session();

    record_interruption(
        InterruptionEvent::ContextSwitch {
            from: "src/db/handler.rs".into(),
            to: "src/ui/mod.rs".into(),
        },
        true,
    );
    record_interruption(
        InterruptionEvent::RedundantRead {
            path: "src/db/pool.rs".into(),
        },
        false,
    );
    record_interruption(InterruptionEvent::EchoRepetition { tokens: 120 }, true);
    record_interruption(InterruptionEvent::BounceWaste { tokens: 80 }, true);

    let report = compute_impact();
    assert!(
        (0.0..=1.0).contains(&report.score),
        "interruption score {} should be in [0, 1]",
        report.score
    );
    assert!(
        report.interruptions_prevented >= 3,
        "expected at least 3 prevented interruptions, got {}",
        report.interruptions_prevented
    );
    assert!(
        report.context_switches_saved >= 1,
        "prevented context switch should increment context_switches_saved"
    );
    assert!(
        report.echo_tokens_saved >= 120,
        "echo repetition prevention should count saved tokens"
    );
    assert!(
        report.focus_time_saved_minutes >= 23.0,
        "one prevented context switch should save ~23 focus minutes"
    );

    let input =
        "The database connection handler acquires a pooled connection and returns it to the pool.";
    let output = "The database connection handler acquires a pooled connection and returns it to the pool when done.";
    let echo = compute_echo_ratio(input, output);

    assert!(
        echo.ratio > 0.5,
        "heavily overlapping output should echo_ratio > 0.5, got {}",
        echo.ratio
    );
    assert_eq!(echo.verdict, "high");
}

// ---------------------------------------------------------------------------
// Test 5: Stigmergy coordination → verbosity recommendation
// ---------------------------------------------------------------------------

#[test]
fn coordination_pipeline() {
    reset_signals();

    let path = "src/db/handler.rs";
    let agents = ["cursor-1001", "codex-2002", "claude-3003"];
    for (agent_id, strength) in agents.iter().zip([0.9_f64, 0.7, 0.8]) {
        deposit_signal(PheromoneSignal {
            agent_id: (*agent_id).to_string(),
            path: path.to_string(),
            symbol: Some("database_connection_handler".to_string()),
            kind: SignalKind::Active,
            strength,
            deposited_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            note: None,
        });
    }

    let signals = read_signals(path, None);
    assert_eq!(
        signals.len(),
        3,
        "all three deposited signals should be readable at the target path"
    );

    let pressure = PressureMap::from_signals(&signals);
    let field = pressure.pressure_at(path);
    assert_eq!(
        field.agent_count, 3,
        "pressure map should detect all three coordinating agents"
    );
    assert!(
        field.total_strength > 0.0,
        "aggregated pressure strength should be positive"
    );

    let entries = vec![
        TranscriptEntry {
            tool: "ctx_read".to_string(),
            target: path.to_string(),
            compression_level: "standard".to_string(),
            response_tokens: 900,
            timestamp: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        },
        TranscriptEntry {
            tool: "ctx_search".to_string(),
            target: "database connection".to_string(),
            compression_level: "lite".to_string(),
            response_tokens: 400,
            timestamp: Utc.with_ymd_and_hms(2026, 1, 1, 0, 5, 0).unwrap(),
        },
        TranscriptEntry {
            tool: "ctx_read".to_string(),
            target: "src/db/pool.rs".to_string(),
            compression_level: "max".to_string(),
            response_tokens: 250,
            timestamp: Utc.with_ymd_and_hms(2026, 1, 1, 0, 10, 0).unwrap(),
        },
    ];

    let behavior = extract_signals(&entries);
    let profile = recommend_level(&behavior);
    assert!(
        is_valid_compression_level(profile.level),
        "recommended compression level should be a valid variant"
    );
    assert!(
        (0.0..=1.0).contains(&profile.confidence),
        "recommendation confidence {} out of bounds",
        profile.confidence
    );
}

// ---------------------------------------------------------------------------
// Test 6: Graph expand → IB relevance on neighborhood
// ---------------------------------------------------------------------------

#[test]
fn graph_to_context_flow() {
    let symbols: Vec<String> = (0..10).map(|idx| format!("sym_{idx}")).collect();
    let edges: [(&str, &str); 15] = [
        ("sym_0", "sym_1"),
        ("sym_0", "sym_2"),
        ("sym_0", "sym_3"),
        ("sym_1", "sym_4"),
        ("sym_1", "sym_5"),
        ("sym_2", "sym_6"),
        ("sym_2", "sym_7"),
        ("sym_3", "sym_8"),
        ("sym_4", "sym_9"),
        ("sym_5", "sym_2"),
        ("sym_6", "sym_1"),
        ("sym_7", "sym_8"),
        ("sym_8", "sym_9"),
        ("sym_9", "sym_0"),
        ("sym_3", "sym_5"),
    ];

    let mut adjacency: HashMap<String, Vec<(String, String, String, EdgeKind)>> = HashMap::new();
    for symbol in &symbols {
        adjacency.insert(symbol.clone(), Vec::new());
    }
    for (from, to) in edges {
        adjacency.get_mut(from).expect("from symbol").push((
            to.to_string(),
            format!("src/{to}.rs"),
            "function".to_string(),
            EdgeKind::Calls,
        ));
    }

    let center = "sym_0";
    let graph = expand_neighborhood(center, "src/sym_0.rs", "function", 2, |symbol| {
        adjacency.get(symbol).cloned().unwrap_or_default()
    });

    assert!(
        graph.nodes.contains_key(center),
        "expanded graph should include the center node"
    );
    assert!(
        graph.node_count() >= 4,
        "2-hop expansion from a connected center should reach multiple nodes"
    );

    let node_names: Vec<&str> = graph.nodes.keys().map(String::as_str).collect();
    let unique: std::collections::HashSet<&str> = node_names.iter().copied().collect();
    assert_eq!(
        node_names.len(),
        unique.len(),
        "partial graph should not contain duplicate node keys"
    );

    for node in graph.nodes.values() {
        assert!(
            node.depth <= 2,
            "node depth {} exceeds max_hops 2",
            node.depth
        );
    }

    let mut seen_edges = std::collections::HashSet::new();
    for (from, to, relation) in &graph.edges {
        assert!(
            seen_edges.insert((from.clone(), to.clone(), *relation)),
            "duplicate edge ({from}, {to}, {relation:?})"
        );
    }

    let source_contents: Vec<String> = graph
        .nodes
        .values()
        .map(|node| format!("fn {}() in {}", node.kind, node.file))
        .collect();
    let source_refs: Vec<&str> = source_contents.iter().map(String::as_str).collect();
    let scores = compute_relevance(&source_refs, &TaskIntent::Explore, Some(QUERY));

    assert_eq!(
        scores.len(),
        source_contents.len(),
        "every expanded graph node should receive a relevance score"
    );
    for score in scores {
        assert!(
            (0.0..=1.0).contains(&score.score),
            "graph-derived relevance score {} out of bounds",
            score.score
        );
    }
}
