use std::collections::HashMap;
use std::path::Path;

use crate::core::graph_index::{self, ProjectIndex};
use crate::core::tokens::count_tokens;

pub fn handle(
    action: &str,
    path: Option<&str>,
    root: &str,
    cache: &mut crate::core::cache::SessionCache,
    crp_mode: crate::tools::CrpMode,
) -> String {
    match action {
        "build" => handle_build(root),
        "related" => handle_related(path, root),
        "symbol" => handle_symbol(path, root, cache, crp_mode),
        "impact" => handle_impact(path, root),
        "status" => handle_status(root),
        _ => "Unknown action. Use: build, related, symbol, impact, status".to_string(),
    }
}

fn handle_build(root: &str) -> String {
    let index = graph_index::scan(root);

    let mut by_lang: HashMap<&str, (usize, usize)> = HashMap::new();
    for entry in index.files.values() {
        let e = by_lang.entry(&entry.language).or_insert((0, 0));
        e.0 += 1;
        e.1 += entry.token_count;
    }

    let mut result = Vec::new();
    result.push(format!(
        "Project Graph: {} files, {} symbols, {} edges",
        index.file_count(),
        index.symbol_count(),
        index.edge_count()
    ));

    let mut langs: Vec<_> = by_lang.iter().collect();
    langs.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
    result.push("\nLanguages:".to_string());
    for (lang, (count, tokens)) in &langs {
        result.push(format!("  {lang}: {count} files, {tokens} tok"));
    }

    let mut import_counts: HashMap<&str, usize> = HashMap::new();
    for edge in &index.edges {
        if edge.kind == "import" {
            *import_counts.entry(&edge.to).or_insert(0) += 1;
        }
    }
    let mut hotspots: Vec<_> = import_counts.iter().collect();
    hotspots.sort_by(|a, b| b.1.cmp(a.1));

    if !hotspots.is_empty() {
        result.push(format!("\nMost imported ({}):", hotspots.len().min(10)));
        for (module, count) in hotspots.iter().take(10) {
            result.push(format!("  {module}: imported by {count} files"));
        }
    }

    if let Some(dir) = ProjectIndex::index_dir(root) {
        result.push(format!(
            "\nIndex saved: {}",
            crate::core::protocol::shorten_path(&dir.to_string_lossy())
        ));
    }

    let output = result.join("\n");
    let tokens = count_tokens(&output);
    format!("{output}\n[ctx_graph build: {tokens} tok]")
}

fn handle_related(path: Option<&str>, root: &str) -> String {
    let target = match path {
        Some(p) => p,
        None => return "path is required for 'related' action".to_string(),
    };

    let index = match ProjectIndex::load(root) {
        Some(idx) => idx,
        None => {
            return "No graph index found. Run ctx_graph with action='build' first.".to_string()
        }
    };

    let rel_target = target
        .strip_prefix(root)
        .unwrap_or(target)
        .trim_start_matches('/');

    let related = index.get_related(rel_target, 2);
    if related.is_empty() {
        return format!(
            "No related files found for {}",
            crate::core::protocol::shorten_path(target)
        );
    }

    let mut result = format!(
        "Files related to {} ({}):\n",
        crate::core::protocol::shorten_path(target),
        related.len()
    );
    for r in &related {
        result.push_str(&format!("  {}\n", crate::core::protocol::shorten_path(r)));
    }

    let tokens = count_tokens(&result);
    format!("{result}[ctx_graph related: {tokens} tok]")
}

fn handle_symbol(
    path: Option<&str>,
    root: &str,
    cache: &mut crate::core::cache::SessionCache,
    crp_mode: crate::tools::CrpMode,
) -> String {
    let spec = match path {
        Some(p) => p,
        None => {
            return "path is required for 'symbol' action (format: file.rs::function_name)"
                .to_string()
        }
    };

    let (file_part, symbol_name) = match spec.split_once("::") {
        Some((f, s)) => (f, s),
        None => return format!("Invalid symbol spec '{spec}'. Use format: file.rs::function_name"),
    };

    let index = match ProjectIndex::load(root) {
        Some(idx) => idx,
        None => {
            return "No graph index found. Run ctx_graph with action='build' first.".to_string()
        }
    };

    let rel_file = file_part
        .strip_prefix(root)
        .unwrap_or(file_part)
        .trim_start_matches('/');

    let key = format!("{rel_file}::{symbol_name}");
    let symbol = match index.get_symbol(&key) {
        Some(s) => s,
        None => {
            let available: Vec<&str> = index
                .symbols
                .keys()
                .filter(|k| k.starts_with(rel_file))
                .map(|k| k.as_str())
                .take(10)
                .collect();
            if available.is_empty() {
                return format!("Symbol '{symbol_name}' not found in {rel_file}. Run ctx_graph action='build' to update the index.");
            }
            return format!(
                "Symbol '{symbol_name}' not found in {rel_file}.\nAvailable symbols:\n  {}",
                available.join("\n  ")
            );
        }
    };

    let abs_path = if Path::new(file_part).is_absolute() {
        file_part.to_string()
    } else {
        format!("{root}/{rel_file}")
    };

    let content = match std::fs::read_to_string(&abs_path) {
        Ok(c) => c,
        Err(e) => return format!("Cannot read {abs_path}: {e}"),
    };

    let lines: Vec<&str> = content.lines().collect();
    let start = symbol.start_line.saturating_sub(1);
    let end = symbol.end_line.min(lines.len());

    if start >= lines.len() {
        return crate::tools::ctx_read::handle(cache, &abs_path, "full", crp_mode);
    }

    let mut result = format!(
        "{}::{} ({}:{}-{})\n",
        crate::core::protocol::shorten_path(rel_file),
        symbol_name,
        symbol.kind,
        symbol.start_line,
        symbol.end_line
    );

    for (i, line) in lines[start..end].iter().enumerate() {
        result.push_str(&format!("{:>4}|{}\n", start + i + 1, line));
    }

    let tokens = count_tokens(&result);
    let full_tokens = count_tokens(&content);
    let saved = full_tokens.saturating_sub(tokens);
    let pct = if full_tokens > 0 {
        (saved as f64 / full_tokens as f64 * 100.0).round() as usize
    } else {
        0
    };

    format!("{result}[ctx_graph symbol: {tokens} tok (full file: {full_tokens} tok, -{pct}%)]")
}

fn handle_impact(path: Option<&str>, root: &str) -> String {
    let target = match path {
        Some(p) => p,
        None => return "path is required for 'impact' action".to_string(),
    };

    let index = match ProjectIndex::load(root) {
        Some(idx) => idx,
        None => {
            return "No graph index found. Run ctx_graph with action='build' first.".to_string()
        }
    };

    let rel_target = target
        .strip_prefix(root)
        .unwrap_or(target)
        .trim_start_matches('/');

    let dependents = index.get_reverse_deps(rel_target, 2);
    if dependents.is_empty() {
        return format!(
            "No files depend on {}",
            crate::core::protocol::shorten_path(target)
        );
    }

    let direct: Vec<&str> = index
        .edges
        .iter()
        .filter(|e| e.to == rel_target && e.kind == "import")
        .map(|e| e.from.as_str())
        .collect();

    let mut result = format!(
        "Impact of {} ({} dependents):\n",
        crate::core::protocol::shorten_path(target),
        dependents.len()
    );

    if !direct.is_empty() {
        result.push_str(&format!("\nDirect ({}):\n", direct.len()));
        for d in &direct {
            result.push_str(&format!("  {}\n", crate::core::protocol::shorten_path(d)));
        }
    }

    let indirect: Vec<&String> = dependents
        .iter()
        .filter(|d| !direct.contains(&d.as_str()))
        .collect();
    if !indirect.is_empty() {
        result.push_str(&format!("\nIndirect ({}):\n", indirect.len()));
        for d in &indirect {
            result.push_str(&format!("  {}\n", crate::core::protocol::shorten_path(d)));
        }
    }

    let tokens = count_tokens(&result);
    format!("{result}[ctx_graph impact: {tokens} tok]")
}

fn handle_status(root: &str) -> String {
    let index = match ProjectIndex::load(root) {
        Some(idx) => idx,
        None => return "No graph index. Run ctx_graph action='build' to create one.".to_string(),
    };

    let mut by_lang: HashMap<&str, usize> = HashMap::new();
    let mut total_tokens = 0usize;
    for entry in index.files.values() {
        *by_lang.entry(&entry.language).or_insert(0) += 1;
        total_tokens += entry.token_count;
    }

    let mut langs: Vec<_> = by_lang.iter().collect();
    langs.sort_by(|a, b| b.1.cmp(a.1));
    let lang_summary: String = langs
        .iter()
        .take(5)
        .map(|(l, c)| format!("{l}:{c}"))
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        "Graph: {} files, {} symbols, {} edges | {} tok total\nLast scan: {}\nLanguages: {lang_summary}\nStored: {}",
        index.file_count(),
        index.symbol_count(),
        index.edge_count(),
        total_tokens,
        index.last_scan,
        ProjectIndex::index_dir(root)
            .map(|d| d.to_string_lossy().to_string())
            .unwrap_or_default()
    )
}
