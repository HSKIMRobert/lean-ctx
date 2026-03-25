use crate::core::litm;
use crate::core::session::SessionState;
use crate::core::stats;

pub struct BenchmarkResult {
    pub scenario: String,
    pub rows: Vec<BenchmarkRow>,
    pub ccp_advantage: String,
}

pub struct BenchmarkRow {
    pub label: String,
    pub tokens: u64,
    pub cost_usd: f64,
    pub litm_efficiency: f64,
}

const COST_PER_TOKEN: f64 = 2.50 / 1_000_000.0;

pub fn run_benchmark(scenario: &str) -> BenchmarkResult {
    match scenario {
        "cold-start" => benchmark_cold_start(),
        "session-resume" => benchmark_session_resume(),
        "litm" => benchmark_litm(),
        _ => BenchmarkResult {
            scenario: scenario.to_string(),
            rows: Vec::new(),
            ccp_advantage: format!("Unknown scenario: {scenario}. Use: cold-start, session-resume, litm"),
        },
    }
}

fn benchmark_cold_start() -> BenchmarkResult {
    let store = stats::load();
    let session = SessionState::load_latest();

    let avg_file_tokens: u64 = if store.total_commands > 0 {
        store.total_input_tokens / store.total_commands
    } else {
        500
    };

    let typical_files = 15u64;
    let raw_tokens = typical_files * avg_file_tokens;
    let raw_litm = compute_litm_for_tokens(raw_tokens);

    let compression_rate = if store.total_input_tokens > 0 {
        1.0 - (store.total_output_tokens as f64 / store.total_input_tokens as f64)
    } else {
        0.6
    };

    let lean_tokens = (raw_tokens as f64 * (1.0 - compression_rate)) as u64;
    let lean_litm = compute_litm_for_tokens(lean_tokens);

    let ccp_tokens = session.as_ref().map_or(400u64, |s| {
        crate::core::tokens::count_tokens(&s.format_compact()) as u64
    });
    let ccp_litm = compute_litm_for_small_context(ccp_tokens);

    let cursorrules_tokens = (raw_tokens as f64 * 0.7) as u64;
    let cursorrules_litm = compute_litm_for_tokens(cursorrules_tokens);

    let advantage_pct = if raw_tokens > 0 {
        (1.0 - ccp_tokens as f64 / raw_tokens as f64) * 100.0
    } else {
        99.0
    };
    let litm_gain = ccp_litm - raw_litm;

    BenchmarkResult {
        scenario: "Cold-Start Context Building".to_string(),
        rows: vec![
            BenchmarkRow {
                label: "Raw (Baseline)".to_string(),
                tokens: raw_tokens,
                cost_usd: raw_tokens as f64 * COST_PER_TOKEN,
                litm_efficiency: raw_litm,
            },
            BenchmarkRow {
                label: ".cursorrules".to_string(),
                tokens: cursorrules_tokens,
                cost_usd: cursorrules_tokens as f64 * COST_PER_TOKEN,
                litm_efficiency: cursorrules_litm,
            },
            BenchmarkRow {
                label: "lean-ctx v1.9".to_string(),
                tokens: lean_tokens,
                cost_usd: lean_tokens as f64 * COST_PER_TOKEN,
                litm_efficiency: lean_litm,
            },
            BenchmarkRow {
                label: "lean-ctx + CCP".to_string(),
                tokens: ccp_tokens,
                cost_usd: ccp_tokens as f64 * COST_PER_TOKEN,
                litm_efficiency: ccp_litm,
            },
        ],
        ccp_advantage: format!(
            "CCP: {advantage_pct:.1}% fewer tokens, +{litm_gain:.1}% LITM efficiency"
        ),
    }
}

fn benchmark_session_resume() -> BenchmarkResult {
    let store = stats::load();
    let session = SessionState::load_latest();

    let avg_rebuild: u64 = if store.total_commands > 0 {
        (store.total_input_tokens / store.total_commands) * 10
    } else {
        50_000
    };

    let compression_rate = if store.total_input_tokens > 0 {
        1.0 - (store.total_output_tokens as f64 / store.total_input_tokens as f64)
    } else {
        0.6
    };

    let lean_tokens = (avg_rebuild as f64 * (1.0 - compression_rate)) as u64;
    let ccp_tokens = session.as_ref().map_or(400u64, |s| {
        crate::core::tokens::count_tokens(&s.format_compact()) as u64
    });

    let advantage_pct = if avg_rebuild > 0 {
        (1.0 - ccp_tokens as f64 / avg_rebuild as f64) * 100.0
    } else {
        99.0
    };

    BenchmarkResult {
        scenario: "Session Resume After Compaction".to_string(),
        rows: vec![
            BenchmarkRow {
                label: "Raw (full rebuild)".to_string(),
                tokens: avg_rebuild,
                cost_usd: avg_rebuild as f64 * COST_PER_TOKEN,
                litm_efficiency: compute_litm_for_tokens(avg_rebuild),
            },
            BenchmarkRow {
                label: "lean-ctx (re-read)".to_string(),
                tokens: lean_tokens,
                cost_usd: lean_tokens as f64 * COST_PER_TOKEN,
                litm_efficiency: compute_litm_for_tokens(lean_tokens),
            },
            BenchmarkRow {
                label: "lean-ctx + CCP".to_string(),
                tokens: ccp_tokens,
                cost_usd: ccp_tokens as f64 * COST_PER_TOKEN,
                litm_efficiency: compute_litm_for_small_context(ccp_tokens),
            },
        ],
        ccp_advantage: format!(
            "CCP: {advantage_pct:.1}% fewer tokens on session resume"
        ),
    }
}

fn benchmark_litm() -> BenchmarkResult {
    let session = SessionState::load_latest();

    let ccp_tokens = session.as_ref().map_or(400u64, |s| {
        crate::core::tokens::count_tokens(&s.format_compact()) as u64
    });

    let scenarios: Vec<(u64, &str)> = vec![
        (10_000, "10K context"),
        (50_000, "50K context"),
        (100_000, "100K context"),
        (200_000, "200K context (max)"),
    ];

    let mut rows = Vec::new();
    for (total, label) in &scenarios {
        let begin = total / 10;
        let end = total / 10;
        let middle = total - begin - end;

        let (eff_without, _) = litm::compute_litm_efficiency(
            begin as usize,
            middle as usize,
            end as usize,
            (begin + ccp_tokens) as usize,
            end as usize,
        );

        let ccp_begin = begin + ccp_tokens;
        let ccp_total = ccp_begin + end;
        let eff_with = (0.9 * ccp_begin as f64 + 0.85 * end as f64) / ccp_total as f64 * 100.0;

        rows.push(BenchmarkRow {
            label: format!("{label} (without CCP)"),
            tokens: *total,
            cost_usd: *total as f64 * COST_PER_TOKEN,
            litm_efficiency: eff_without,
        });
        rows.push(BenchmarkRow {
            label: format!("{label} (with CCP)"),
            tokens: ccp_begin + end,
            cost_usd: (ccp_begin + end) as f64 * COST_PER_TOKEN,
            litm_efficiency: eff_with,
        });
    }

    BenchmarkResult {
        scenario: "LITM Efficiency Analysis".to_string(),
        rows,
        ccp_advantage: "CCP eliminates the lossy middle, placing info at attention-optimal positions (Liu et al., 2023)".to_string(),
    }
}

pub fn format_benchmark(result: &BenchmarkResult) -> String {
    let mut lines = Vec::new();
    lines.push(format!("BENCHMARK: {}", result.scenario));
    lines.push("\u{2550}".repeat(60));

    lines.push(format!(
        "{:<30} {:>10} {:>10} {:>10}",
        "", "Tokens", "Cost", "LITM-Eff"
    ));

    for row in &result.rows {
        lines.push(format!(
            "{:<30} {:>10} {:>10} {:>9.1}%",
            row.label,
            format_tokens(row.tokens),
            format!("${:.3}", row.cost_usd),
            row.litm_efficiency,
        ));
    }

    lines.push("\u{2550}".repeat(60));
    lines.push(result.ccp_advantage.clone());
    lines.join("\n")
}

fn compute_litm_for_tokens(total: u64) -> f64 {
    if total == 0 { return 0.0; }
    let begin = total / 10;
    let end_tok = total / 10;
    let middle = total - begin - end_tok;
    let effective = 0.9 * begin as f64 + 0.55 * middle as f64 + 0.85 * end_tok as f64;
    effective / total as f64 * 100.0
}

fn compute_litm_for_small_context(total: u64) -> f64 {
    if total == 0 { return 0.0; }
    let half = total / 2;
    let other = total - half;
    let effective = 0.9 * half as f64 + 0.85 * other as f64;
    effective / total as f64 * 100.0
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{tokens}")
    }
}
