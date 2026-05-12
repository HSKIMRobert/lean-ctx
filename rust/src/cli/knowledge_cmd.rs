use crate::tools::ctx_knowledge;

pub(crate) fn cmd_knowledge(args: &[String]) {
    let project_root = super::common::detect_project_root(args);
    let action = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str);

    match action {
        Some("remember") => cmd_remember(args, &project_root),
        Some("recall") => cmd_recall(args, &project_root),
        Some("search") => cmd_search(args),
        Some("export") => cmd_export(args, &project_root),
        Some("remove") => cmd_remove(args, &project_root),
        Some("import") => cmd_import(args, &project_root),
        Some("status") => {
            #[cfg(unix)]
            {
                #[cfg(unix)]
                if let Some(out) = crate::daemon_client::try_daemon_tool_call_blocking_text(
                    "ctx_knowledge",
                    Some(serde_json::json!({
                        "action": "status",
                        "project_root": project_root,
                    })),
                ) {
                    println!("{out}");
                    return;
                }
            }
            let out = ctx_knowledge::handle(
                &project_root,
                "status",
                None,
                None,
                None,
                None,
                &cli_session_id(),
                None,
                None,
                None,
                None,
            );
            println!("{out}");
        }
        Some("health") => {
            #[cfg(unix)]
            {
                #[cfg(unix)]
                if let Some(out) = crate::daemon_client::try_daemon_tool_call_blocking_text(
                    "ctx_knowledge",
                    Some(serde_json::json!({
                        "action": "health",
                        "project_root": project_root,
                    })),
                ) {
                    println!("{out}");
                    return;
                }
            }
            let out = ctx_knowledge::handle(
                &project_root,
                "health",
                None,
                None,
                None,
                None,
                &cli_session_id(),
                None,
                None,
                None,
                None,
            );
            println!("{out}");
        }
        _ => {
            print_help();
            if action.is_some() {
                std::process::exit(1);
            }
        }
    }
}

fn cmd_remember(args: &[String], project_root: &str) {
    let category = value_arg(args, "--category").or_else(|| value_arg(args, "-c"));
    let key = value_arg(args, "--key").or_else(|| value_arg(args, "-k"));
    let confidence = value_arg(args, "--confidence").and_then(|v| v.parse::<f32>().ok());

    let value = positional_after(args, "remember");

    if category.is_none() || key.is_none() || value.is_none() {
        eprintln!(
            "Usage: lean-ctx knowledge remember <value> --category <cat> --key <key> [--confidence <0.0-1.0>]"
        );
        eprintln!("Example: lean-ctx knowledge remember \"Uses JWT for auth\" --category auth --key token-type");
        std::process::exit(1);
    }

    #[cfg(unix)]
    {
        #[cfg(unix)]
        if let Some(out) = crate::daemon_client::try_daemon_tool_call_blocking_text(
            "ctx_knowledge",
            Some(serde_json::json!({
                "action": "remember",
                "project_root": project_root,
                "category": category,
                "key": key,
                "value": value,
                "confidence": confidence,
            })),
        ) {
            println!("{out}");
            return;
        }
    }

    let out = ctx_knowledge::handle(
        project_root,
        "remember",
        category.as_deref(),
        key.as_deref(),
        value.as_deref(),
        None,
        &cli_session_id(),
        None,
        None,
        confidence,
        None,
    );
    println!("{out}");
}

fn cmd_recall(args: &[String], project_root: &str) {
    let category = value_arg(args, "--category").or_else(|| value_arg(args, "-c"));
    let mode = value_arg(args, "--mode").or_else(|| value_arg(args, "-m"));
    let query = positional_after(args, "recall");

    if category.is_none() && query.is_none() {
        eprintln!("Usage: lean-ctx knowledge recall [query] [--category <cat>] [--mode auto|semantic|hybrid]");
        eprintln!("Example: lean-ctx knowledge recall \"auth\" --category security");
        std::process::exit(1);
    }

    #[cfg(unix)]
    {
        #[cfg(unix)]
        if let Some(out) = crate::daemon_client::try_daemon_tool_call_blocking_text(
            "ctx_knowledge",
            Some(serde_json::json!({
                "action": "recall",
                "project_root": project_root,
                "category": category,
                "query": query,
                "mode": mode,
            })),
        ) {
            println!("{out}");
            return;
        }
    }

    let out = ctx_knowledge::handle(
        project_root,
        "recall",
        category.as_deref(),
        None,
        None,
        query.as_deref(),
        &cli_session_id(),
        None,
        None,
        None,
        mode.as_deref(),
    );
    println!("{out}");
}

fn cmd_search(args: &[String]) {
    let query = positional_after(args, "search");

    if query.is_none() {
        eprintln!("Usage: lean-ctx knowledge search <query>");
        eprintln!("Example: lean-ctx knowledge search \"authentication\"");
        std::process::exit(1);
    }

    #[cfg(unix)]
    {
        #[cfg(unix)]
        if let Some(out) = crate::daemon_client::try_daemon_tool_call_blocking_text(
            "ctx_knowledge",
            Some(serde_json::json!({
                "action": "search",
                "query": query,
            })),
        ) {
            println!("{out}");
            return;
        }
    }

    let out = ctx_knowledge::handle(
        "",
        "search",
        None,
        None,
        None,
        query.as_deref(),
        &cli_session_id(),
        None,
        None,
        None,
        None,
    );
    println!("{out}");
}

fn cmd_export(args: &[String], project_root: &str) {
    let format = value_arg(args, "--format")
        .or_else(|| value_arg(args, "-f"))
        .unwrap_or_else(|| "json".into());
    let output = value_arg(args, "--output").or_else(|| value_arg(args, "-o"));

    let Some(knowledge) = crate::core::knowledge::ProjectKnowledge::load(project_root) else {
        eprintln!("No knowledge stored for this project yet.");
        std::process::exit(1);
    };

    let content = match format.as_str() {
        "json" => match serde_json::to_string_pretty(&knowledge) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Export failed: {e}");
                std::process::exit(1);
            }
        },
        "jsonl" => {
            let entries = knowledge.export_simple();
            entries
                .iter()
                .filter_map(|e| serde_json::to_string(e).ok())
                .collect::<Vec<_>>()
                .join("\n")
        }
        "simple" => match serde_json::to_string_pretty(&knowledge.export_simple()) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Export failed: {e}");
                std::process::exit(1);
            }
        },
        _ => {
            eprintln!("Unknown format: {format}. Use: json, jsonl, simple");
            std::process::exit(1);
        }
    };

    if let Some(path) = output {
        let p = std::path::Path::new(&path);
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match crate::config_io::write_atomic_with_backup(p, &content) {
            Ok(()) => {
                let active = knowledge.facts.iter().filter(|f| f.is_current()).count();
                eprintln!("Exported to {path} ({active} active facts, format={format})");
            }
            Err(e) => {
                eprintln!("Failed to write {path}: {e}");
                std::process::exit(1);
            }
        }
    } else {
        println!("{content}");
    }
}

fn cmd_remove(args: &[String], project_root: &str) {
    let category = value_arg(args, "--category").or_else(|| value_arg(args, "-c"));
    let key = value_arg(args, "--key").or_else(|| value_arg(args, "-k"));

    if category.is_none() || key.is_none() {
        eprintln!("Usage: lean-ctx knowledge remove --category <cat> --key <key>");
        eprintln!("Example: lean-ctx knowledge remove --category auth --key token-type");
        std::process::exit(1);
    }

    #[cfg(unix)]
    {
        #[cfg(unix)]
        if let Some(out) = crate::daemon_client::try_daemon_tool_call_blocking_text(
            "ctx_knowledge",
            Some(serde_json::json!({
                "action": "remove",
                "project_root": project_root,
                "category": category,
                "key": key,
            })),
        ) {
            println!("{out}");
            return;
        }
    }

    let out = ctx_knowledge::handle(
        project_root,
        "remove",
        category.as_deref(),
        key.as_deref(),
        None,
        None,
        &cli_session_id(),
        None,
        None,
        None,
        None,
    );
    println!("{out}");
}

fn cmd_import(args: &[String], project_root: &str) {
    let path = positional_after(args, "import");
    let merge_str = value_arg(args, "--merge")
        .or_else(|| value_arg(args, "-m"))
        .unwrap_or_else(|| "skip-existing".into());
    let dry_run = args.iter().any(|a| a == "--dry-run");

    let Some(path) = path else {
        eprintln!(
            "Usage: lean-ctx knowledge import <path> [--merge replace|append|skip-existing] [--dry-run]"
        );
        eprintln!("Formats accepted: native JSON, simple JSON array, JSONL");
        std::process::exit(1);
    };

    let Some(merge) = crate::core::knowledge::ImportMerge::parse(&merge_str) else {
        eprintln!("Unknown merge strategy: {merge_str}. Use: replace, append, skip-existing");
        std::process::exit(1);
    };

    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read {path}: {e}");
            std::process::exit(1);
        }
    };

    let facts = match crate::core::knowledge::parse_import_data(&data) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Parse error: {e}");
            std::process::exit(1);
        }
    };

    let total = facts.len();
    println!("Parsed {total} facts from {path}");

    if dry_run {
        let knowledge = crate::core::knowledge::ProjectKnowledge::load_or_create(project_root);
        let mut would_add = 0u32;
        let mut would_skip = 0u32;
        let mut would_replace = 0u32;

        for fact in &facts {
            let exists = knowledge
                .facts
                .iter()
                .any(|f| f.category == fact.category && f.key == fact.key && f.is_current());
            match (&merge, exists) {
                (crate::core::knowledge::ImportMerge::SkipExisting, true) => would_skip += 1,
                (crate::core::knowledge::ImportMerge::Replace, true) => would_replace += 1,
                (crate::core::knowledge::ImportMerge::Append, true) | (_, false) => {
                    would_add += 1;
                }
            }
        }

        println!("[DRY RUN] Would add: {would_add}, skip: {would_skip}, replace: {would_replace}");
        for fact in facts.iter().take(10) {
            println!(
                "  [{}/{}]: {}",
                fact.category,
                fact.key,
                &fact.value[..fact.value.len().min(80)]
            );
        }
        if total > 10 {
            println!("  ... and {} more", total - 10);
        }
        return;
    }

    let policy = match crate::tools::knowledge_shared::load_policy_or_error() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let mut knowledge = crate::core::knowledge::ProjectKnowledge::load_or_create(project_root);
    let session_id = cli_session_id();
    let result = knowledge.import_facts(facts, merge, &session_id, &policy);

    match knowledge.save() {
        Ok(()) => {
            println!(
                "Import complete: {} added, {} skipped, {} replaced (merge={})",
                result.added, result.skipped, result.replaced, merge_str
            );
        }
        Err(e) => {
            eprintln!(
                "Import done ({} added, {} skipped, {} replaced) but save failed: {e}",
                result.added, result.skipped, result.replaced
            );
            std::process::exit(1);
        }
    }
}

fn cli_session_id() -> String {
    format!("cli-{}", &uuid_short())
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{ts:x}")
}

fn value_arg(args: &[String], key: &str) -> Option<String> {
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

fn positional_after(args: &[String], subcommand: &str) -> Option<String> {
    let mut found_sub = false;
    for a in args {
        if !found_sub {
            if a == subcommand {
                found_sub = true;
            }
            continue;
        }
        if a.starts_with("--") || a.starts_with("-c") || a.starts_with("-k") || a.starts_with("-m")
        {
            continue;
        }
        // Skip the value that follows a flag like --category <val>
        let prev = args
            .iter()
            .position(|x| std::ptr::eq(x, a))
            .and_then(|i| i.checked_sub(1))
            .map(|i| &args[i]);
        if let Some(p) = prev {
            if p.starts_with("--") || p == "-c" || p == "-k" || p == "-m" {
                continue;
            }
        }
        return Some(a.clone());
    }
    None
}

fn print_help() {
    eprintln!(
        "\
lean-ctx knowledge — Project knowledge base

Usage:
  lean-ctx knowledge remember <value> --category <cat> --key <key> [--confidence <0-1>]
  lean-ctx knowledge recall [query] [--category <cat>] [--mode auto|semantic|hybrid]
  lean-ctx knowledge search <query>
  lean-ctx knowledge export [--format json|jsonl|simple] [--output <path>]
  lean-ctx knowledge import <path> [--merge replace|append|skip-existing] [--dry-run]
  lean-ctx knowledge remove --category <cat> --key <key>
  lean-ctx knowledge status
  lean-ctx knowledge health

Examples:
  lean-ctx knowledge remember \"Uses JWT tokens\" --category auth --key token-type
  lean-ctx knowledge recall \"authentication\"
  lean-ctx knowledge export --format jsonl --output backup.jsonl
  lean-ctx knowledge import backup.json --merge skip-existing --dry-run
  lean-ctx knowledge remove --category auth --key token-type
  lean-ctx knowledge status"
    );
}
