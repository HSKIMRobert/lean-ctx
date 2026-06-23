//! `lean-ctx doctor overhead` — honest fixed-cost accounting (#572).
//!
//! Shows what a session costs BEFORE lean-ctx saves anything:
//!  1. advertised MCP tool schemas (mirrors the live `tools/list` policy),
//!  2. the MCP server instructions block,
//!  3. every rules file a client auto-loads, with duplicate detection.
//!
//! Research context: fixed context costs both money and model attention
//! (context degradation starts well below typical window limits), so every
//! always-on token has to justify itself.
//!
//! The rules-file enumeration lives in [`crate::core::rules_overhead`] so the
//! `lean-ctx tools health` report (#848) can reuse the exact same accounting.

use std::path::PathBuf;

use crate::core::context_overhead::tool_tokens;
use crate::core::rules_overhead::{RulesFileCost, collect_rules_files, duplicate_clients};
use crate::core::tokens::count_tokens;

const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RST: &str = "\x1b[0m";

#[derive(Debug, serde::Serialize)]
pub(super) struct OverheadReport {
    pub tool_count: usize,
    pub tool_schema_tokens: usize,
    pub lean_default_tool_count: usize,
    pub lean_default_tool_tokens: usize,
    pub tool_profile: String,
    pub instruction_tokens: usize,
    pub rules_files: Vec<RulesFileCost>,
    pub duplicate_clients: Vec<(String, usize)>,
}

impl OverheadReport {
    fn rules_tokens_total(&self) -> usize {
        self.rules_files.iter().map(|r| r.file_tokens).sum()
    }

    fn total_tokens(&self) -> usize {
        self.tool_schema_tokens + self.instruction_tokens + self.rules_tokens_total()
    }
}

#[must_use]
pub(super) fn measure(home: &std::path::Path, project: &std::path::Path) -> OverheadReport {
    let cfg = crate::core::config::Config::load();
    let advertised = crate::server::tool_visibility::advertised_tool_defs_default();
    let lean_default = crate::tool_defs::lazy_tool_defs();

    let instructions = crate::instructions::build_instructions(crate::tools::CrpMode::effective());

    let rules_files = collect_rules_files(home, project);
    let duplicates = duplicate_clients(&rules_files);

    let pinned = crate::server::tool_visibility::explicit_profile(&cfg);
    let tool_profile = if pinned {
        cfg.tool_profile_effective().as_str().to_string()
    } else {
        "lean (default)".to_string()
    };

    OverheadReport {
        tool_count: advertised.len(),
        tool_schema_tokens: advertised.iter().map(tool_tokens).sum(),
        lean_default_tool_count: lean_default.len(),
        lean_default_tool_tokens: lean_default.iter().map(tool_tokens).sum(),
        tool_profile,
        instruction_tokens: count_tokens(&instructions),
        rules_files,
        duplicate_clients: duplicates,
    }
}

pub(super) fn run_overhead(json: bool) -> i32 {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    let project = std::env::current_dir().unwrap_or_else(|_| home.clone());
    let report = measure(&home, &project);

    if json {
        match serde_json::to_string_pretty(&report) {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("doctor overhead: JSON serialization failed: {e}");
                return 2;
            }
        }
        return 0;
    }

    println!("{BOLD}Fixed context overhead per session{RST}");
    println!("{DIM}What every session pays before any compression saves a token.{RST}\n");

    // 1. Tool schemas
    println!(
        "  {BOLD}MCP tool schemas{RST}      {:>6} tok  {DIM}({} tools advertised, profile: {}){RST}",
        report.tool_schema_tokens, report.tool_count, report.tool_profile
    );
    if report.tool_count > report.lean_default_tool_count {
        let saving = report
            .tool_schema_tokens
            .saturating_sub(report.lean_default_tool_tokens);
        println!(
            "  {YELLOW}→ lean default advertises {} tools ({} tok) — `lean-ctx tools lean` saves ~{saving} tok/session{RST}",
            report.lean_default_tool_count, report.lean_default_tool_tokens
        );
    }

    // 2. Instructions
    println!(
        "  {BOLD}MCP instructions{RST}      {:>6} tok",
        report.instruction_tokens
    );

    // 3. Rules files
    println!(
        "  {BOLD}Rules files{RST}           {:>6} tok  {DIM}({} auto-loaded files){RST}",
        report.rules_tokens_total(),
        report.rules_files.len()
    );
    for f in &report.rules_files {
        let ours = if f.lean_ctx_tokens == 0 {
            String::new()
        } else if f.carries_full {
            format!(", {} tok lean-ctx", f.lean_ctx_tokens)
        } else {
            format!(", {} tok pointer", f.lean_ctx_tokens)
        };
        println!(
            "    {DIM}{:<58}{RST} {:>6} tok  {DIM}[{}{}]{RST}",
            shorten(&f.path, 58),
            f.file_tokens,
            f.clients.join("+"),
            ours
        );
    }

    if !report.duplicate_clients.is_empty() {
        println!();
        for (client, n) in &report.duplicate_clients {
            println!(
                "  {YELLOW}⚠ {client}: {n} files contain lean-ctx rules — the same guidance is billed {n}× per session.{RST}"
            );
        }
        println!(
            "  {DIM}Fix: `lean-ctx rules dedup --apply` keeps one canonical source per client (#578).{RST}"
        );
    }

    println!();
    let total = report.total_tokens();
    let color = if total > 8000 { YELLOW } else { GREEN };
    println!("  {BOLD}Total fixed cost{RST}      {color}{total:>6} tok / session{RST}");
    println!(
        "  {DIM}With provider prompt caching, repeated turns re-bill this at ~10% — but only if the prefix stays byte-stable.{RST}"
    );

    0
}

fn shorten(path: &str, max: usize) -> String {
    if path.len() <= max {
        return path.to_string();
    }
    let tail: String = path
        .chars()
        .rev()
        .take(max.saturating_sub(1))
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("…{tail}")
}
