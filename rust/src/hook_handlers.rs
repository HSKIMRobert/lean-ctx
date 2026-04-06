use std::io::Read;

pub fn handle_rewrite() {
    let binary = resolve_binary();
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    let tool = extract_json_field(&input, "tool_name");
    if !matches!(tool.as_deref(), Some("Bash" | "bash")) {
        return;
    }

    let cmd = match extract_json_field(&input, "command") {
        Some(c) => c,
        None => return,
    };

    if cmd.starts_with("lean-ctx ") || cmd.starts_with(&format!("{binary} ")) {
        return;
    }

    let should_rewrite = REWRITABLE_PREFIXES
        .iter()
        .any(|prefix| cmd.starts_with(prefix) || cmd == prefix.trim_end_matches(' '));

    if should_rewrite {
        let rewrite = format!("{binary} -c {cmd}");
        print!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"allow\",\"updatedInput\":{{\"command\":\"{rewrite}\"}}}}}}"
        );
    }
}

pub fn handle_redirect() {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    let tool = match extract_json_field(&input, "tool_name") {
        Some(t) => t,
        None => return,
    };

    let reason = match tool.as_str() {
        "Read" | "read" | "ReadFile" | "read_file" | "View" | "view" => {
            "STOP. Use ctx_read(path) from the lean-ctx MCP server instead. \
             It saves 60-80% input tokens via caching and compression. \
             Available modes: full, map, signatures, diff, lines:N-M. \
             Never use native Read — always use ctx_read."
        }
        "Grep" | "grep" | "Search" | "search" | "RipGrep" | "ripgrep" => {
            "STOP. Use ctx_search(pattern, path) from the lean-ctx MCP server instead. \
             It provides compact, token-efficient results with .gitignore awareness. \
             Never use native Grep — always use ctx_search."
        }
        "ListFiles" | "list_files" | "ListDirectory" | "list_directory" => {
            "STOP. Use ctx_tree(path, depth) from the lean-ctx MCP server instead. \
             It provides compact directory maps with file counts. \
             Never use native ListFiles — always use ctx_tree."
        }
        _ => return,
    };

    if !is_lean_ctx_running() {
        return;
    }

    print!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"{reason}\"}}}}"
    );
}

const REWRITABLE_PREFIXES: &[&str] = &[
    "git ", "gh ", "cargo ", "npm ", "pnpm ", "yarn ", "docker ", "kubectl ", "pip ", "pip3 ",
    "ruff ", "go ", "curl ", "grep ", "rg ", "find ", "cat ", "head ", "tail ", "ls ", "ls",
    "aws ", "helm ", "eslint", "prettier", "tsc", "pytest", "mypy",
];

fn resolve_binary() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "lean-ctx".to_string())
}

fn is_lean_ctx_running() -> bool {
    if cfg!(windows) {
        std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq lean-ctx.exe", "/NH"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("lean-ctx"))
            .unwrap_or(false)
    } else {
        std::process::Command::new("pgrep")
            .args(["-f", "lean-ctx"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

fn extract_json_field(input: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", field);
    let start = input.find(&pattern)? + pattern.len();
    let rest = &input[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
