use crate::tools::ctx_knowledge;

pub fn cmd_knowledge(args: &[String]) {
    let project_root = super::common::detect_project_root(args);
    let action = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str);

    match action {
        Some("remember") => cmd_remember(args, &project_root),
        Some("recall") => cmd_recall(args, &project_root),
        Some("search") => cmd_search(args),
        Some("status") => {
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
  lean-ctx knowledge status
  lean-ctx knowledge health

Examples:
  lean-ctx knowledge remember \"Uses JWT tokens\" --category auth --key token-type
  lean-ctx knowledge recall \"authentication\"
  lean-ctx knowledge recall --category security
  lean-ctx knowledge search \"database migration\"
  lean-ctx knowledge status"
    );
}
