use crate::{core, tools};

pub(super) fn cmd_gain(rest: &[String]) {
    if rest.iter().any(|a| a == "--reset") {
        core::stats::reset_all();
        println!("Stats reset. All token savings data cleared.");
        return;
    }
    if rest.iter().any(|a| a == "--live" || a == "--watch") {
        core::stats::gain_live();
        return;
    }
    let model = rest.iter().enumerate().find_map(|(i, a)| {
        if let Some(v) = a.strip_prefix("--model=") {
            return Some(v.to_string());
        }
        if a == "--model" {
            return rest.get(i + 1).cloned();
        }
        None
    });
    let period = rest
        .iter()
        .enumerate()
        .find_map(|(i, a)| {
            if let Some(v) = a.strip_prefix("--period=") {
                return Some(v.to_string());
            }
            if a == "--period" {
                return rest.get(i + 1).cloned();
            }
            None
        })
        .unwrap_or_else(|| "all".to_string());
    let limit = rest
        .iter()
        .enumerate()
        .find_map(|(i, a)| {
            if let Some(v) = a.strip_prefix("--limit=") {
                return v.parse::<usize>().ok();
            }
            if a == "--limit" {
                return rest.get(i + 1).and_then(|v| v.parse::<usize>().ok());
            }
            None
        })
        .unwrap_or(10);

    let copy = rest.iter().any(|a| a == "--copy");
    let open = rest.iter().any(|a| a == "--open");

    if let Some(req) = unpublish_request(rest) {
        let id = match &req {
            UnpublishReq::Id(s) => Some(s.as_str()),
            UnpublishReq::Latest => None,
        };
        crate::cli::wrapped_publish::unpublish(id);
        return;
    }
    if rest.iter().any(|a| a == "--publish") {
        let leaderboard = rest.iter().any(|a| a == "--leaderboard");
        crate::cli::wrapped_publish::publish(&period, name_arg(rest).as_deref(), leaderboard);
        return;
    }

    if let Some(svg_path) = svg_target(rest) {
        let report = core::wrapped::WrappedReport::generate(&period);
        match std::fs::write(&svg_path, report.to_svg()) {
            Ok(()) => println!(
                "Wrapped card written to {svg_path}\n\
                 Share it directly, open it in a browser, or convert to PNG with any SVG tool."
            ),
            Err(e) => {
                eprintln!("Failed to write {svg_path}: {e}");
                std::process::exit(1);
            }
        }
        share_side_effects(&report, Some(&svg_path), copy, open);
        return;
    }

    if let Some(html_path) = share_target(rest) {
        let report = core::wrapped::WrappedReport::generate(&period);
        let base = base_url_arg(rest);
        match std::fs::write(&html_path, report.to_share_html(base.as_deref())) {
            Ok(()) => {
                println!("Share page written to {html_path}");
                if base.is_some() {
                    println!(
                        "Host it at your base URL; for social preview cards, place a PNG \
                         (rasterise the SVG) at <base>/lean-ctx-wrapped.png."
                    );
                } else {
                    println!(
                        "Self-contained (SVG embedded) — host it anywhere to get a permalink. \
                         Pass --base-url=https://… to emit social preview meta."
                    );
                }
            }
            Err(e) => {
                eprintln!("Failed to write {html_path}: {e}");
                std::process::exit(1);
            }
        }
        share_side_effects(&report, Some(&html_path), copy, open);
        return;
    }

    if copy {
        let report = core::wrapped::WrappedReport::generate(&period);
        share_side_effects(&report, None, true, false);
        return;
    }

    if rest.iter().any(|a| a == "--graph") {
        println!("{}", core::stats::format_gain_graph());
    } else if rest.iter().any(|a| a == "--daily") {
        println!("{}", core::stats::format_gain_daily());
    } else if rest.iter().any(|a| a == "--json") {
        println!(
            "{}",
            tools::ctx_gain::handle("json", Some(&period), model.as_deref(), Some(limit))
        );
    } else if rest.iter().any(|a| a == "--score") {
        println!(
            "{}",
            tools::ctx_gain::handle("score", None, model.as_deref(), Some(limit))
        );
    } else if rest.iter().any(|a| a == "--cost") {
        println!(
            "{}",
            tools::ctx_gain::handle("cost", None, model.as_deref(), Some(limit))
        );
    } else if rest.iter().any(|a| a == "--tasks") {
        println!(
            "{}",
            tools::ctx_gain::handle("tasks", None, None, Some(limit))
        );
    } else if rest.iter().any(|a| a == "--agents") {
        println!(
            "{}",
            tools::ctx_gain::handle("agents", None, None, Some(limit))
        );
    } else if rest.iter().any(|a| a == "--heatmap") {
        println!(
            "{}",
            tools::ctx_gain::handle("heatmap", None, None, Some(limit))
        );
    } else if rest.iter().any(|a| a == "--wrapped") {
        println!(
            "{}",
            tools::ctx_gain::handle("wrapped", Some(&period), model.as_deref(), Some(limit))
        );
        crate::cli::wrapped_publish::maybe_auto_publish(&period);
    } else if rest.iter().any(|a| a == "--pipeline") {
        let stats_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".lean-ctx")
            .join("pipeline_stats.json");
        if let Ok(data) = std::fs::read_to_string(&stats_path) {
            if let Ok(stats) = serde_json::from_str::<core::pipeline::PipelineStats>(&data) {
                println!("{}", stats.format_summary());
            } else {
                println!("No pipeline stats available yet (corrupt data).");
            }
        } else {
            println!("No pipeline stats available yet. Use MCP tools to generate data.");
        }
    } else if rest.iter().any(|a| a == "--deep") {
        println!(
            "{}\n{}\n{}\n{}\n{}",
            tools::ctx_gain::handle("report", None, model.as_deref(), Some(limit)),
            tools::ctx_gain::handle("tasks", None, None, Some(limit)),
            tools::ctx_gain::handle("cost", None, model.as_deref(), Some(limit)),
            tools::ctx_gain::handle("agents", None, None, Some(limit)),
            tools::ctx_gain::handle("heatmap", None, None, Some(limit))
        );
    } else {
        println!("{}", core::stats::format_gain());
        crate::cli::wrapped_publish::maybe_auto_publish(&period);
    }
}

/// Resolves the output path for the shareable SVG Wrapped card, or `None` when no
/// card was requested. Accepts `--svg`, `--svg=<path>`, `--card`, `--card=<path>`;
/// a bare flag falls back to `lean-ctx-wrapped.svg` in the current directory.
/// A requested `--unpublish`: either an explicit card id, or the most recent published card.
enum UnpublishReq {
    Latest,
    Id(String),
}

/// Parses `--unpublish[=<id>]`. `None` means it was not requested at all.
fn unpublish_request(rest: &[String]) -> Option<UnpublishReq> {
    for a in rest {
        if let Some(v) = a.strip_prefix("--unpublish=") {
            return Some(UnpublishReq::Id(v.to_string()));
        }
        if a == "--unpublish" {
            return Some(UnpublishReq::Latest);
        }
    }
    None
}

/// Parses `--name=<display>` / `--name <display>` for the optional publish display label.
fn name_arg(rest: &[String]) -> Option<String> {
    rest.iter().enumerate().find_map(|(i, a)| {
        if let Some(v) = a.strip_prefix("--name=") {
            return Some(v.to_string());
        }
        if a == "--name" {
            return rest.get(i + 1).cloned();
        }
        None
    })
}

fn svg_target(rest: &[String]) -> Option<String> {
    let mut requested = false;
    let mut path: Option<String> = None;
    for (i, a) in rest.iter().enumerate() {
        if let Some(v) = a
            .strip_prefix("--svg=")
            .or_else(|| a.strip_prefix("--card="))
        {
            requested = true;
            path = Some(v.to_string());
        } else if a == "--svg" || a == "--card" {
            requested = true;
            if let Some(next) = rest.get(i + 1) {
                if !next.starts_with('-') {
                    path = Some(next.clone());
                }
            }
        }
    }
    requested.then(|| path.unwrap_or_else(|| "lean-ctx-wrapped.svg".to_string()))
}

/// Resolves the output path for the self-hostable HTML share page, or `None` when not
/// requested. Accepts `--share`, `--share=<path>`, `--page`, `--page=<path>`; a bare flag
/// falls back to `lean-ctx-wrapped.html`.
fn share_target(rest: &[String]) -> Option<String> {
    let mut requested = false;
    let mut path: Option<String> = None;
    for (i, a) in rest.iter().enumerate() {
        if let Some(v) = a
            .strip_prefix("--share=")
            .or_else(|| a.strip_prefix("--page="))
        {
            requested = true;
            path = Some(v.to_string());
        } else if a == "--share" || a == "--page" {
            requested = true;
            if let Some(next) = rest.get(i + 1) {
                if !next.starts_with('-') {
                    path = Some(next.clone());
                }
            }
        }
    }
    requested.then(|| path.unwrap_or_else(|| "lean-ctx-wrapped.html".to_string()))
}

/// Reads `--base-url=<url>` / `--base-url <url>` (the location the share page will be
/// hosted at, used for absolute social-preview image meta).
fn base_url_arg(rest: &[String]) -> Option<String> {
    rest.iter().enumerate().find_map(|(i, a)| {
        if let Some(v) = a.strip_prefix("--base-url=") {
            return Some(v.to_string());
        }
        if a == "--base-url" {
            return rest.get(i + 1).cloned();
        }
        None
    })
}

/// Applies the optional `--copy` (clipboard) and `--open` (browser) side effects for a
/// Wrapped artifact. `path` is the just-written file to open, when one exists. Both
/// degrade gracefully: a clipboard miss falls back to printing the share line.
fn share_side_effects(
    report: &core::wrapped::WrappedReport,
    path: Option<&str>,
    copy: bool,
    open: bool,
) {
    if copy {
        let text = report.share_text(None);
        if core::share::copy_to_clipboard(&text) {
            println!("Copied to clipboard:  {text}");
        } else {
            println!("Share text (copy it): {text}");
        }
    }
    if open {
        if let Some(p) = path {
            if core::share::open_in_browser(p) {
                println!("Opened {p}");
            } else {
                println!("Could not open {p} automatically — open it manually.");
            }
        }
    }
}

pub(super) fn cmd_savings(rest: &[String]) {
    let action = rest.first().map_or("summary", String::as_str);
    match action {
        "verify" => {
            let v = core::savings_ledger::verify();
            if v.valid {
                println!(
                    "Savings ledger: OK — {} event(s), SHA-256 chain intact.",
                    v.total
                );
            } else {
                println!(
                    "Savings ledger: TAMPERED — chain broke at entry {} (of {} verified).",
                    v.first_invalid_at.unwrap_or(0),
                    v.total
                );
                std::process::exit(1);
            }
        }
        "export" => {
            let events = core::savings_ledger::all_events();
            match serde_json::to_string_pretty(&events) {
                Ok(json) => println!("{json}"),
                Err(e) => {
                    eprintln!("Export failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        "sign" => cmd_savings_sign(&rest[1..]),
        "verify-batch" => cmd_savings_verify_batch(rest.get(1).map(String::as_str)),
        "summary" | "" => print!("{}", format_savings_summary()),
        _ => {
            eprintln!("Usage: lean-ctx savings [summary|verify|export|sign|verify-batch]");
            std::process::exit(1);
        }
    }
}

/// Resolves the signing identity (same precedence as the ledger's own attribution).
fn savings_agent_id() -> String {
    std::env::var("LEAN_CTX_AGENT_ID")
        .or_else(|_| std::env::var("LCTX_AGENT_ID"))
        .unwrap_or_else(|_| "local".to_string())
}

/// `lean-ctx savings sign [--out FILE]` — builds + Ed25519-signs a portable savings batch and
/// writes the JSON artifact (offline-verifiable with `savings verify-batch`).
fn cmd_savings_sign(args: &[String]) {
    use core::savings_ledger::{signed_batch, SignedSavingsBatchV1};

    let out_override = args.iter().find_map(|a| {
        a.strip_prefix("--out=")
            .map(std::path::PathBuf::from)
            .or_else(|| (a == "--out").then_some(std::path::PathBuf::new()))
    });
    // Support `--out FILE` (space-separated) too.
    let out_override = match out_override {
        Some(p) if p.as_os_str().is_empty() => args
            .iter()
            .skip_while(|a| a.as_str() != "--out")
            .nth(1)
            .map(std::path::PathBuf::from),
        other => other,
    };

    let agent_id = savings_agent_id();
    let mut batch = SignedSavingsBatchV1::build_all(&agent_id);
    if batch.totals.total_events == 0 {
        eprintln!("Savings ledger is empty — nothing to sign yet. It fills as lean-ctx compresses your reads.");
        std::process::exit(1);
    }
    if let Err(e) = batch.sign(&agent_id) {
        eprintln!("Signing failed: {e}");
        std::process::exit(1);
    }

    let out = match out_override {
        Some(p) => Ok(p),
        None => signed_batch::default_artifact_path(),
    };
    let path = out.and_then(|p| signed_batch::write_artifact(&batch, &p));
    match path {
        Ok(p) => {
            use core::wrapped::format_tokens;
            let pk = batch.signer_public_key.as_deref().unwrap_or("");
            println!("Signed savings batch written to {}", p.display());
            println!(
                "  Net saved:  {} tokens (~${:.2}) over {} event(s)",
                format_tokens(batch.totals.net_saved_tokens),
                batch.totals.saved_usd,
                batch.totals.total_events
            );
            println!("  Chain head: {}", batch.last_entry_hash);
            println!(
                "  Chain:      {}",
                if batch.chain_valid {
                    "intact (SHA-256)"
                } else {
                    "BROKEN — run `lean-ctx savings verify`"
                }
            );
            println!("  Signer key: {pk}");
            println!(
                "\nVerify anywhere (no ledger needed):  lean-ctx savings verify-batch {}",
                p.display()
            );
        }
        Err(e) => {
            eprintln!("Could not write artifact: {e}");
            std::process::exit(1);
        }
    }
}

/// `lean-ctx savings verify-batch FILE` — verifies an exported batch's Ed25519 signature
/// offline (integrity + origin), without needing the original ledger.
fn cmd_savings_verify_batch(file: Option<&str>) {
    use core::savings_ledger::signed_batch;

    let Some(file) = file else {
        eprintln!("Usage: lean-ctx savings verify-batch <file.json>");
        std::process::exit(1);
    };
    let batch = match signed_batch::load_artifact(std::path::Path::new(file)) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Cannot read artifact: {e}");
            std::process::exit(1);
        }
    };
    let res = batch.verify();
    if res.signature_valid {
        use core::wrapped::format_tokens;
        println!("Signed savings batch: VALID");
        println!(
            "  Signed by:  {}",
            res.signer_public_key.as_deref().unwrap_or("?")
        );
        println!("  Agent:      {}", batch.agent_id);
        println!("  Created:    {}", batch.created_at);
        println!("  lean-ctx:   {}", batch.lean_ctx_version);
        println!(
            "  Net saved:  {} tokens (~${:.2}) over {} event(s)",
            format_tokens(batch.totals.net_saved_tokens),
            batch.totals.saved_usd,
            batch.totals.total_events
        );
        println!("  Chain head: {}", batch.last_entry_hash);
        if !batch.chain_valid {
            println!("  NOTE: the ledger chain was already broken when this batch was signed.");
        }
    } else {
        println!(
            "Signed savings batch: INVALID — {}",
            res.error.as_deref().unwrap_or("signature check failed")
        );
        std::process::exit(1);
    }
}

fn format_savings_summary() -> String {
    use core::wrapped::format_tokens;
    let s = core::savings_ledger::summary();
    if s.total_events == 0 {
        return "Savings ledger is empty — it fills as lean-ctx compresses your reads.\n"
            .to_string();
    }
    let v = core::savings_ledger::verify();
    // Net saved tokens drive the energy estimate (same J/token basis as the /metrics board).
    let energy_tokens = if s.bounce_events > 0 {
        s.net_saved_tokens()
    } else {
        s.saved_tokens
    };
    let mut out = String::new();
    out.push_str("Verified Savings Ledger (local, auditable)\n");
    out.push_str("──────────────────────────────────────────\n");
    out.push_str(&format!("Events:        {}\n", s.total_events));
    if s.bounce_events > 0 {
        out.push_str(&format!(
            "Saved tokens:  {}  (gross)\n",
            format_tokens(s.saved_tokens)
        ));
        out.push_str(&format!(
            "Bounce:        {}  ({} compressed->full re-read(s))\n",
            format_tokens(s.bounce_tokens),
            s.bounce_events
        ));
        out.push_str(&format!(
            "Net saved:     {}\n",
            format_tokens(s.net_saved_tokens())
        ));
        out.push_str(&format!(
            "Net (USD):     ${:.2}  (net of bounce; excludes prompt-cache discounts)\n",
            s.saved_usd
        ));
    } else {
        out.push_str(&format!(
            "Saved tokens:  {}\n",
            format_tokens(s.saved_tokens)
        ));
        out.push_str(&format!(
            "Saved (USD):   ${:.2}  (upper bound, model input price)\n",
            s.saved_usd
        ));
    }
    {
        let energy = core::energy::format_for_tokens(energy_tokens);
        let charges = core::energy::phone_charges_hint(energy_tokens)
            .map(|h| format!("  ({h})"))
            .unwrap_or_default();
        out.push_str(&format!("Energy saved:  {energy}{charges}  (est.)\n"));
    }
    if !s.tokenizers.is_empty() {
        out.push_str(&format!("Tokenizer:     {}\n", s.tokenizers.join(", ")));
    }
    out.push_str(&format!(
        "Integrity:     {}\n",
        if v.valid {
            "SHA-256 chain intact"
        } else {
            "BROKEN — run `lean-ctx savings verify`"
        }
    ));
    if !s.by_model.is_empty() {
        out.push_str("\nBy model:\n");
        for (model, tok, usd) in s.by_model.iter().take(5) {
            out.push_str(&format!(
                "  {model:<22} {:>10} tok  ${usd:.2}\n",
                format_tokens(*tok)
            ));
        }
    }
    if s.by_day.len() >= 2 {
        out.push_str("\nRecent days:\n");
        let recent: Vec<_> = s.by_day.iter().rev().take(7).collect();
        for (day, tok, usd) in recent.into_iter().rev() {
            out.push_str(&format!(
                "  {day}  {:>10} tok  ${usd:.2}\n",
                format_tokens(*tok)
            ));
        }
    }
    out
}

pub(super) fn cmd_graph(rest: &[String]) {
    let sub = rest.first().map_or("build", std::string::String::as_str);
    match sub {
        "build" => {
            let root = rest.get(1).cloned().or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            });
            let root = root.unwrap_or_else(|| ".".to_string());
            let index = core::graph_index::load_or_build(&root);
            println!(
                "Graph built: {} files, {} edges",
                index.files.len(),
                index.edges.len()
            );
        }
        "export-html" => {
            let mut root: Option<String> = None;
            let mut out: Option<String> = None;
            let mut max_nodes: usize = 2500;

            let args = &rest[1..];
            let mut i = 0usize;
            while i < args.len() {
                let a = args[i].as_str();
                if let Some(v) = a.strip_prefix("--root=") {
                    root = Some(v.to_string());
                } else if a == "--root" {
                    root = args.get(i + 1).cloned();
                    i += 1;
                } else if let Some(v) = a.strip_prefix("--out=") {
                    out = Some(v.to_string());
                } else if a == "--out" {
                    out = args.get(i + 1).cloned();
                    i += 1;
                } else if let Some(v) = a.strip_prefix("--max-nodes=") {
                    max_nodes = v.parse::<usize>().unwrap_or(0);
                } else if a == "--max-nodes" {
                    let v = args.get(i + 1).map_or("", String::as_str);
                    max_nodes = v.parse::<usize>().unwrap_or(0);
                    i += 1;
                }
                i += 1;
            }

            let root = root
                .or_else(|| {
                    std::env::current_dir()
                        .ok()
                        .map(|p| p.to_string_lossy().to_string())
                })
                .unwrap_or_else(|| ".".to_string());
            let Some(out) = out else {
                eprintln!("Usage: lean-ctx graph export-html --out <path> [--root <path>] [--max-nodes <n>]");
                std::process::exit(1);
            };
            if max_nodes == 0 {
                eprintln!("--max-nodes must be >= 1");
                std::process::exit(1);
            }

            core::graph_export::export_graph_html(&root, std::path::Path::new(&out), max_nodes)
                .unwrap_or_else(|e| {
                    eprintln!("graph export failed: {e}");
                    std::process::exit(1);
                });
            println!("{out}");
        }
        "related" | "impact" | "symbol" | "context" | "status" => {
            let path_arg = if sub == "status" {
                None
            } else {
                rest.get(1).map(String::as_str)
            };
            let root_idx = if sub == "status" { 1 } else { 2 };
            let root = resolve_graph_root(rest.get(root_idx));
            println!(
                "{}",
                tools::ctx_graph::handle(
                    sub,
                    path_arg,
                    &root,
                    &mut core::cache::SessionCache::new(),
                    tools::CrpMode::Off,
                    None,
                    None,
                )
            );
        }
        _ => {
            eprintln!(
                "Usage:\n  \
                 lean-ctx graph build [path]\n  \
                 lean-ctx graph related <file>\n  \
                 lean-ctx graph impact <file|symbol>\n  \
                 lean-ctx graph symbol <name>\n  \
                 lean-ctx graph context <query>\n  \
                 lean-ctx graph status\n  \
                 lean-ctx graph export-html --out <path> [--root <path>] [--max-nodes <n>]"
            );
            std::process::exit(1);
        }
    }
}

pub(super) fn cmd_smells(rest: &[String]) {
    let action = rest.first().map_or("summary", String::as_str);
    let rule = rest.iter().enumerate().find_map(|(i, a)| {
        if let Some(v) = a.strip_prefix("--rule=") {
            return Some(v.to_string());
        }
        if a == "--rule" {
            return rest.get(i + 1).cloned();
        }
        None
    });
    let path = rest.iter().enumerate().find_map(|(i, a)| {
        if let Some(v) = a.strip_prefix("--path=") {
            return Some(v.to_string());
        }
        if a == "--path" {
            return rest.get(i + 1).cloned();
        }
        None
    });
    let root = rest
        .iter()
        .enumerate()
        .find_map(|(i, a)| {
            if let Some(v) = a.strip_prefix("--root=") {
                return Some(v.to_string());
            }
            if a == "--root" {
                return rest.get(i + 1).cloned();
            }
            None
        })
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| ".".to_string());
    let fmt = if rest.iter().any(|a| a == "--json") {
        Some("json")
    } else {
        None
    };
    println!(
        "{}",
        tools::ctx_smells::handle(action, rule.as_deref(), path.as_deref(), &root, fmt)
    );
}

fn resolve_graph_root(arg: Option<&String>) -> String {
    arg.cloned()
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| ".".to_string())
}

pub(super) fn cmd_compact(rest: &[String]) {
    let target = rest.first().map_or_else(
        || {
            let home = dirs::home_dir().unwrap_or_default();
            let claude = home.join(".claude").join("projects");
            if claude.is_dir() {
                claude
            } else {
                let cursor = home.join(".cursor").join("agent-transcripts");
                if cursor.is_dir() {
                    cursor
                } else {
                    std::env::current_dir().unwrap_or_default()
                }
            }
        },
        std::path::PathBuf::from,
    );

    if !target.exists() {
        eprintln!("Path does not exist: {}", target.display());
        std::process::exit(1);
    }

    let result = if target.is_file() {
        core::transcript_compact::compact_file(&target)
    } else {
        core::transcript_compact::compact_directory(&target)
    };

    match result {
        Ok(stats) => {
            println!("Transcript compaction: {stats}");
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{base_url_arg, share_target, svg_target};

    fn args(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn no_flag_means_no_card() {
        assert_eq!(svg_target(&args(&["--wrapped", "--period=month"])), None);
    }

    #[test]
    fn bare_flag_uses_default_path() {
        assert_eq!(
            svg_target(&args(&["--svg"])).as_deref(),
            Some("lean-ctx-wrapped.svg")
        );
        assert_eq!(
            svg_target(&args(&["--card"])).as_deref(),
            Some("lean-ctx-wrapped.svg")
        );
    }

    #[test]
    fn explicit_path_is_used() {
        assert_eq!(
            svg_target(&args(&["--svg=out.svg"])).as_deref(),
            Some("out.svg")
        );
        assert_eq!(
            svg_target(&args(&["--card=c.svg"])).as_deref(),
            Some("c.svg")
        );
        assert_eq!(
            svg_target(&args(&["--svg", "chosen.svg"])).as_deref(),
            Some("chosen.svg")
        );
    }

    #[test]
    fn following_flag_is_not_consumed_as_path() {
        assert_eq!(
            svg_target(&args(&["--svg", "--period=week"])).as_deref(),
            Some("lean-ctx-wrapped.svg")
        );
    }

    #[test]
    fn share_target_resolves_paths() {
        assert_eq!(share_target(&args(&["--wrapped"])), None);
        assert_eq!(
            share_target(&args(&["--share"])).as_deref(),
            Some("lean-ctx-wrapped.html")
        );
        assert_eq!(
            share_target(&args(&["--page=out.html"])).as_deref(),
            Some("out.html")
        );
        assert_eq!(
            share_target(&args(&["--share", "--base-url=https://x"])).as_deref(),
            Some("lean-ctx-wrapped.html"),
            "a following flag must not be eaten as the path"
        );
    }

    #[test]
    fn base_url_is_parsed_both_forms() {
        assert_eq!(
            base_url_arg(&args(&["--base-url=https://me.dev"])).as_deref(),
            Some("https://me.dev")
        );
        assert_eq!(
            base_url_arg(&args(&["--base-url", "https://me.dev"])).as_deref(),
            Some("https://me.dev")
        );
        assert_eq!(base_url_arg(&args(&["--share"])), None);
    }
}
