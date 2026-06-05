//! `lean-ctx repomap` — PageRank-ranked repo map for the CLI & editors.
//!
//! Same ranking as the `ctx_repomap` MCP tool. `--json` emits a per-file array
//! `[{path, rank, symbols[]}]` (what the VS Code / Cursor extensions consume);
//! otherwise the budget-fitted human-readable map is printed.

use crate::core::repomap::{self, ranking::RankedSymbol};

const DEFAULT_MAX_TOKENS: usize = 2048;
const DEFAULT_MAX_FILES: usize = 100;
const MAX_SYMBOLS_PER_FILE: usize = 25;

pub(crate) fn cmd_repomap(args: &[String]) {
    let json = args.iter().any(|a| a == "--json");
    let project_root = super::common::detect_project_root(args);
    let max_tokens = flag_value(args, "--max-tokens")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_TOKENS);
    let max_files = flag_value(args, "--limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_FILES);

    let graph = repomap::RepoGraph::build(&project_root);
    if graph.files.is_empty() {
        if json {
            println!("[]");
        } else {
            println!(
                "No indexable files found in '{project_root}'. \
                 Ensure it contains source files (.rs, .ts, .py, etc.)."
            );
        }
        return;
    }

    let ranked = repomap::rank_symbols(&graph, &[], &[]);
    if json {
        println!("{}", to_json(&ranked, max_files));
    } else {
        println!("{}", repomap::fit_to_budget(&ranked, max_tokens));
    }
}

/// Aggregates ranked symbols into per-file entries: file rank is the highest
/// symbol score in that file, symbols are the file's symbol names in rank order
/// (deduped). Files are returned highest-rank first. `{path,rank,symbols}` is
/// the contract the editor extensions depend on.
fn to_json(ranked: &[RankedSymbol], max_files: usize) -> String {
    use std::collections::HashMap;

    // `ranked` is already sorted by descending score, so per-file symbol order
    // is preserved as we encounter them.
    let mut order: Vec<&str> = Vec::new();
    let mut by_file: HashMap<&str, (f64, Vec<&str>)> = HashMap::new();
    for r in ranked {
        let file = r.def.file.as_str();
        let name = r.def.name.as_str();
        if let Some((rank, symbols)) = by_file.get_mut(file) {
            if r.score > *rank {
                *rank = r.score;
            }
            if !symbols.contains(&name) {
                symbols.push(name);
            }
        } else {
            order.push(file);
            by_file.insert(file, (r.score, vec![name]));
        }
    }

    order.sort_by(|a, b| {
        by_file[b]
            .0
            .partial_cmp(&by_file[a].0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(b))
    });

    #[derive(serde::Serialize)]
    struct Entry<'a> {
        path: &'a str,
        rank: f64,
        symbols: Vec<&'a str>,
    }

    let out: Vec<Entry> = order
        .iter()
        .take(max_files)
        .map(|f| {
            let (rank, symbols) = &by_file[f];
            Entry {
                path: f,
                rank: *rank,
                symbols: symbols.iter().take(MAX_SYMBOLS_PER_FILE).copied().collect(),
            }
        })
        .collect();

    serde_json::to_string(&out).unwrap_or_else(|_| "[]".to_string())
}

fn flag_value(args: &[String], key: &str) -> Option<String> {
    for (i, a) in args.iter().enumerate() {
        if let Some(v) = a.strip_prefix(&format!("{key}=")) {
            return Some(v.to_string());
        }
        if a == key {
            return args.get(i + 1).cloned();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::repomap::graph::SymbolDef;

    fn sym(name: &str, file: &str, exported: bool) -> SymbolDef {
        SymbolDef {
            name: name.into(),
            kind: "fn".into(),
            file: file.into(),
            line: 1,
            end_line: 5,
            is_exported: exported,
            signature: format!("fn {name}"),
        }
    }

    fn ranked(name: &str, file: &str, score: f64) -> RankedSymbol {
        RankedSymbol {
            def: sym(name, file, true),
            score,
        }
    }

    #[test]
    fn aggregates_per_file_and_sorts_by_rank() {
        let input = vec![
            ranked("high", "b.rs", 0.9),
            ranked("mid", "a.rs", 0.5),
            ranked("also_b", "b.rs", 0.7),
        ];
        let json = to_json(&input, 100);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(v.as_array().unwrap().len(), 2, "two distinct files");
        // b.rs (max 0.9) ranks before a.rs (0.5).
        assert_eq!(v[0]["path"], "b.rs");
        assert_eq!(v[0]["rank"], 0.9);
        assert_eq!(v[1]["path"], "a.rs");

        let b_syms: Vec<&str> = v[0]["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        assert_eq!(b_syms, vec!["high", "also_b"]);
    }

    #[test]
    fn dedups_repeated_symbol_names() {
        let input = vec![ranked("dup", "a.rs", 0.5), ranked("dup", "a.rs", 0.4)];
        let json = to_json(&input, 100);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v[0]["symbols"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn empty_input_is_empty_array() {
        assert_eq!(to_json(&[], 100), "[]");
    }

    #[test]
    fn respects_max_files() {
        let input = vec![
            ranked("s1", "a.rs", 0.9),
            ranked("s2", "b.rs", 0.8),
            ranked("s3", "c.rs", 0.7),
        ];
        let json = to_json(&input, 2);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[test]
    fn flag_value_parses_space_and_equals() {
        let a: Vec<String> = ["--limit", "5", "--max-tokens=999"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        assert_eq!(flag_value(&a, "--limit").as_deref(), Some("5"));
        assert_eq!(flag_value(&a, "--max-tokens").as_deref(), Some("999"));
    }
}
