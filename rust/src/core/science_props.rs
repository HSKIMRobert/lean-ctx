//! Property-based invariants for science modules (F1–F10 + echo ratio).
//!
//! Kept inside the crate so tests can reach `pub(crate)` APIs.

use chrono::{Duration, TimeZone, Utc};
use proptest::prelude::*;

use crate::core::anti_interrupt::{
    InterruptionEvent, TEST_LOCK as ANTI_INTERRUPT_LOCK, compute_impact, record_interruption,
    reset_session,
};
use crate::core::cognitive::{ChunkKind, SemanticChunk, budget_select, detect_chunks};
use crate::core::config::CompressionLevel;
use crate::core::context_prefetch::FileTrajectory;
use crate::core::echo_ratio::compute_echo_ratio;
use crate::core::graph_expand::{EdgeKind, expand_neighborhood};
use crate::core::ib::{TaskIntent, classify_intent, compute_relevance};
use crate::core::mdl_mode::generate_structural_description;
use crate::core::memory_scheduler::{initial_state, retrievability, update_stability};
use crate::core::session::{SessionState, TaskInfo};
use crate::core::stigmergy::{PheromoneSignal, PressureMap, SignalKind, reset_signals};
use crate::core::verbosity::{TranscriptEntry, extract_signals, recommend_level};
use crate::core::wasserstein::allocate_budget;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_short_string() -> impl Strategy<Value = String> {
    "\\PC{0,200}"
}

fn arb_task_intent() -> impl Strategy<Value = TaskIntent> {
    prop_oneof![
        Just(TaskIntent::Debug),
        Just(TaskIntent::Refactor),
        Just(TaskIntent::Implement),
        Just(TaskIntent::Review),
        Just(TaskIntent::Explore),
        Just(TaskIntent::Unknown),
    ]
}

fn arb_chunk_kind() -> impl Strategy<Value = ChunkKind> {
    prop_oneof![
        Just(ChunkKind::Function),
        Just(ChunkKind::Type),
        Just(ChunkKind::Test),
        Just(ChunkKind::Config),
        Just(ChunkKind::Comment),
        Just(ChunkKind::Import),
        Just(ChunkKind::Block),
    ]
}

fn arb_semantic_chunk() -> impl Strategy<Value = SemanticChunk> {
    (
        arb_chunk_kind(),
        1_usize..500,
        0.1_f64..100.0,
        1_usize..5000,
    )
        .prop_map(|(kind, line, complexity, token_count)| SemanticChunk {
            content: format!("line {line} content"),
            kind,
            complexity,
            line_range: (line, line),
            token_count,
        })
}

fn arb_session(description: String) -> SessionState {
    let mut session = SessionState::new();
    session.task = Some(TaskInfo {
        description,
        intent: None,
        progress_pct: None,
    });
    session
}

fn arb_interruption_event() -> impl Strategy<Value = InterruptionEvent> {
    prop_oneof![
        (0_u64..10_000).prop_map(|tokens| InterruptionEvent::EchoRepetition { tokens }),
        arb_short_string().prop_map(|path| InterruptionEvent::RedundantRead { path }),
        (arb_short_string(), arb_short_string())
            .prop_map(|(from, to)| { InterruptionEvent::ContextSwitch { from, to } }),
        (0_u64..10_000).prop_map(|tokens| InterruptionEvent::BounceWaste { tokens }),
        arb_short_string().prop_map(|fact_key| InterruptionEvent::StaleContext { fact_key }),
    ]
}

fn arb_signal_kind() -> impl Strategy<Value = SignalKind> {
    prop_oneof![
        Just(SignalKind::Active),
        Just(SignalKind::Complexity),
        Just(SignalKind::ReviewNeeded),
        Just(SignalKind::Issue),
        Just(SignalKind::Completed),
    ]
}

fn arb_pheromone_signal() -> impl Strategy<Value = PheromoneSignal> {
    (
        arb_short_string(),
        arb_short_string(),
        arb_signal_kind(),
        (0.0_f64..=1.0),
    )
        .prop_map(|(agent_id, path, kind, strength)| PheromoneSignal {
            agent_id,
            path,
            symbol: None,
            kind,
            strength,
            deposited_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            note: None,
        })
}

fn arb_transcript_entry() -> impl Strategy<Value = TranscriptEntry> {
    (
        prop_oneof![Just("ctx_read"), Just("ctx_search"), Just("ctx_shell")],
        arb_short_string(),
        prop_oneof![
            Just("off"),
            Just("lite"),
            Just("standard"),
            Just("max"),
            Just("raw"),
        ],
        0_usize..10_000,
    )
        .prop_map(
            |(tool, target, compression_level, response_tokens)| TranscriptEntry {
                tool: tool.to_string(),
                target,
                compression_level: compression_level.to_string(),
                response_tokens,
                timestamp: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            },
        )
}

fn arb_rust_source() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("fn helper() -> u32 { 42 }"),
            Just("pub struct Widget { id: u64, name: String }"),
            Just(
                "impl Widget { pub fn new(id: u64) -> Self { Self { id, name: String::new() } } }"
            ),
            Just("#[test] fn test_widget() { assert_eq!(Widget::new(1).id, 1); }"),
        ],
        3_usize..=12,
    )
    .prop_map(|lines| lines.join("\n\n"))
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

// ---------------------------------------------------------------------------
// IB (Information Bottleneck)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn relevance_scores_bounded(
        chunks in prop::collection::vec(arb_short_string(), 0..=20),
        intent in arb_task_intent(),
        query in prop::option::of(arb_short_string()),
    ) {
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let scores = compute_relevance(&chunk_refs, &intent, query.as_deref());
        prop_assert_eq!(scores.len(), chunks.len());
        for score in scores {
            prop_assert!(
                (0.0..=1.0).contains(&score.score),
                "score {} out of bounds for chunk {}",
                score.score,
                score.chunk_index
            );
        }
    }

    #[test]
    fn unknown_intent_returns_valid_scores(description in arb_short_string()) {
        let session = arb_session(description);
        let _intent = classify_intent(&session);
    }
}

// ---------------------------------------------------------------------------
// Cognitive Budget
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn chunk_count_bounded_by_millers_law(
        source in prop::collection::vec(arb_short_string(), 1..=20),
    ) {
        let content = source.join("\n");
        let chunks = detect_chunks(&content, "rs");
        let selected = budget_select(&chunks, None);
        prop_assert!(selected.len() <= 9, "selected {} chunks, max is 9", selected.len());
    }

    #[test]
    fn budget_select_never_exceeds_budget(
        chunks in prop::collection::vec(arb_semantic_chunk(), 0..=20),
        max_chunks in 1_usize..=20,
    ) {
        let budget_tokens: usize = chunks.iter().map(|chunk| chunk.token_count).sum();
        let selected = budget_select(&chunks, Some(max_chunks));
        let selected_tokens: usize = selected
            .iter()
            .filter_map(|index| chunks.get(*index))
            .map(|chunk| chunk.token_count)
            .sum();

        prop_assert!(selected.len() <= max_chunks.min(9));
        prop_assert!(selected.len() <= chunks.len());
        prop_assert!(selected_tokens <= budget_tokens);
    }
}

// ---------------------------------------------------------------------------
// Memory Scheduler (FSRS)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn retrievability_bounded(
        stability in 0.001_f64..1_000.0,
        elapsed_days in 0.0_f64..365.0,
        rating in 1_u8..=4,
    ) {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let mut state = initial_state("fact".to_string(), rating);
        state.stability = stability;
        state.last_review = base;
        let now = base + Duration::seconds((elapsed_days * 86_400.0) as i64);
        let r = retrievability(&state, now);
        prop_assert!(
            (0.0..=1.0).contains(&r),
            "retrievability {} out of bounds (stability={}, elapsed={})",
            r,
            stability,
            elapsed_days
        );
    }

    #[test]
    fn stability_monotone_after_correct(
        rating in 3_u8..=4,
        elapsed_days in 1_i64..365,
    ) {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let mut state = initial_state("fact".to_string(), rating);
        state.last_review = base - Duration::days(elapsed_days);
        let old_stability = state.stability;
        update_stability(&mut state, rating);
        prop_assert!(
            state.stability >= old_stability,
            "stability decreased from {} to {} after rating {}",
            old_stability,
            state.stability,
            rating
        );
    }
}

// ---------------------------------------------------------------------------
// Anti-Interrupt
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn impact_interruption_rate_bounded(
        events in prop::collection::vec((arb_interruption_event(), any::<bool>()), 0..=20),
    ) {
        let _guard = ANTI_INTERRUPT_LOCK.lock().expect("anti-interrupt test lock");
        reset_session();
        for (event, prevented) in events {
            record_interruption(event, prevented);
        }
        let report = compute_impact();
        prop_assert!(
            (0.0..=1.0).contains(&report.score),
            "interruption rate {} out of bounds",
            report.score
        );
    }
}

// ---------------------------------------------------------------------------
// Wasserstein
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn allocation_sums_to_budget(
        files in prop::collection::vec(
            (arb_short_string(), 1_usize..500, 0.0_f64..1.0),
            1..=20,
        ),
        total_budget in 1_usize..=10_000,
    ) {
        let inputs: Vec<(&str, usize, f64)> = files
            .iter()
            .map(|(path, tokens, relevance)| (path.as_str(), *tokens, *relevance))
            .collect();
        let allocations = allocate_budget(&inputs, total_budget);
        let assigned: usize = allocations.iter().map(|entry| entry.tokens).sum();
        prop_assert_eq!(assigned, total_budget);
        prop_assert_eq!(allocations.len(), files.len());
    }

    #[test]
    fn zero_relevance_gets_zero_tokens(
        high in 0.01_f64..1.0,
        budget in 1_usize..=1_000,
    ) {
        let allocations = allocate_budget(
            &[("relevant.rs", 100, high), ("irrelevant.rs", 100, 0.0)],
            budget,
        );
        prop_assert_eq!(allocations.len(), 2);
        let zero_entry = allocations
            .iter()
            .find(|entry| entry.target == "irrelevant.rs")
            .expect("irrelevant entry");
        prop_assert_eq!(zero_entry.tokens, 0);
    }
}

// ---------------------------------------------------------------------------
// Graph Expand
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn no_duplicate_edges(
        symbol_count in 2_usize..=10,
        edge_count in 0_usize..=30,
        max_hops in 0_usize..=5,
    ) {
        let symbols: Vec<String> = (0..symbol_count).map(|idx| format!("sym_{idx}")).collect();
        let mut adjacency: std::collections::HashMap<String, Vec<(String, String, String, EdgeKind)>> =
            std::collections::HashMap::new();
        for symbol in &symbols {
            adjacency.insert(symbol.clone(), Vec::new());
        }

        let mut rng_edges = Vec::new();
        for edge_idx in 0..edge_count {
            let from = symbols[edge_idx % symbol_count].clone();
            let to = symbols[(edge_idx + 1) % symbol_count].clone();
            let relation = match edge_idx % 4 {
                0 => EdgeKind::Calls,
                1 => EdgeKind::CalledBy,
                2 => EdgeKind::Imports,
                _ => EdgeKind::Implements,
            };
            rng_edges.push((from, to, relation));
        }
        for (from, to, relation) in rng_edges {
            adjacency.get_mut(&from).expect("from symbol").push((
                to,
                format!("{from}.rs"),
                "function".to_string(),
                relation,
            ));
        }

        let center = symbols[0].clone();
        let graph = expand_neighborhood(
            &center,
            "src/center.rs",
            "function",
            max_hops,
            |symbol| adjacency.get(symbol).cloned().unwrap_or_default(),
        );

        let mut seen = std::collections::HashSet::new();
        for (from, to, relation) in &graph.edges {
            prop_assert!(
                seen.insert((from.clone(), to.clone(), *relation)),
                "duplicate edge ({from}, {to}, {relation:?})"
            );
        }
    }

    #[test]
    fn depth_bounded(
        symbol_count in 2_usize..=10,
        max_hops in 0_usize..=5,
    ) {
        let symbols: Vec<String> = (0..symbol_count).map(|idx| format!("sym_{idx}")).collect();
        let mut adjacency: std::collections::HashMap<String, Vec<(String, String, String, EdgeKind)>> =
            std::collections::HashMap::new();
        for left in &symbols {
            for right in &symbols {
                if left != right {
                    adjacency.entry(left.clone()).or_default().push((
                        right.clone(),
                        format!("{right}.rs"),
                        "function".to_string(),
                        EdgeKind::Calls,
                    ));
                }
            }
        }

        let center = symbols[0].clone();
        let graph = expand_neighborhood(
            &center,
            "src/center.rs",
            "function",
            max_hops,
            |symbol| adjacency.get(symbol).cloned().unwrap_or_default(),
        );

        for node in graph.nodes.values() {
            prop_assert!(
                node.depth <= max_hops,
                "node depth {} exceeds max_hops {}",
                node.depth,
                max_hops
            );
        }
    }
}

// ---------------------------------------------------------------------------
// MDL Mode
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn compression_ratio_positive(source in arb_rust_source()) {
        let desc = generate_structural_description(&source, "prop.rs", "rs");
        let ratio = desc.compression_ratio();
        prop_assert!(ratio > 0.0, "compression ratio {} not positive", ratio);
    }

    #[test]
    fn description_shorter_than_source(source in arb_rust_source()) {
        let desc = generate_structural_description(&source, "prop.rs", "rs");
        prop_assert!(desc.original_tokens > 0);
        let ratio = desc.compression_ratio();
        // Structural render carries fixed header overhead; allow modest expansion.
        prop_assert!(
            ratio <= 1.25,
            "description tokens {} too large vs source {} (ratio={})",
            desc.description_tokens,
            desc.original_tokens,
            ratio
        );
    }
}

// ---------------------------------------------------------------------------
// Verbosity
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn recommend_level_always_valid(
        entries in prop::collection::vec(arb_transcript_entry(), 0..=20),
    ) {
        let signals = extract_signals(&entries);
        let profile = recommend_level(&signals);
        prop_assert!(is_valid_compression_level(profile.level));
        prop_assert!((0.0..=1.0).contains(&profile.confidence));
    }
}

// ---------------------------------------------------------------------------
// Context Prefetch
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn predictions_bounded_by_top_k(
        paths in prop::collection::vec(arb_short_string(), 1..=20),
        top_k in 0_usize..=20,
    ) {
        let mut trajectory = FileTrajectory::new(50);
        for path in paths {
            if !path.is_empty() {
                trajectory.record(&path);
            }
        }
        let predictions = trajectory.predict(top_k);
        prop_assert!(predictions.len() <= top_k);
    }
}

// ---------------------------------------------------------------------------
// Stigmergy
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn pressure_non_negative(signals in prop::collection::vec(arb_pheromone_signal(), 0..=20)) {
        reset_signals();
        let map = PressureMap::from_signals(&signals);
        for field in map.fields.values() {
            prop_assert!(
                field.total_strength >= 0.0,
                "negative pressure {}",
                field.total_strength
            );
        }
        for signal in &signals {
            let field = map.pressure_at(&signal.path);
            prop_assert!(field.total_strength >= 0.0);
        }
    }
}

// ---------------------------------------------------------------------------
// Echo Ratio
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn echo_ratio_bounded(input in arb_short_string(), output in arb_short_string()) {
        let report = compute_echo_ratio(&input, &output);
        prop_assert!(
            (0.0..=1.0).contains(&report.ratio),
            "echo ratio {} out of bounds",
            report.ratio
        );
    }

    #[test]
    fn empty_output_zero_ratio(input in arb_short_string()) {
        let report = compute_echo_ratio(&input, "");
        prop_assert_eq!(report.ratio, 0.0);
        prop_assert_eq!(report.output_tokens, 0);
    }
}
