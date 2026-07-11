use std::io::Read;

use super::HOOK_STDIN_TIMEOUT;

const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "bmp", "svg", "pdf", "zip", "tar", "gz", "bz2",
    "xz", "7z", "rar", "woff", "woff2", "ttf", "otf", "eot", "mp3", "mp4", "wav", "avi", "mov",
    "mkv", "so", "dylib", "dll", "exe", "bin", "o", "a", "class", "pyc", "wasm",
];

/// Handle the `lean-ctx hook deny` subcommand.
///
/// Called by PreToolUse hooks in Replace mode. Denies native Read/Grep/Glob/Shell
/// calls unless an exception applies (binary files, MCP down, etc.).
///
/// Output format matches both Claude Code and Cursor hook protocols.
pub fn handle_deny() {
    let stdin_payload = read_stdin_with_timeout();
    let tool_name = extract_tool_name(&stdin_payload);
    let file_path = extract_file_path(&stdin_payload);

    if should_allow(&tool_name, file_path.as_deref()) {
        print_allow();
    } else {
        print_deny(&tool_name);
    }
}

fn should_allow(tool_name: &str, file_path: Option<&str>) -> bool {
    if super::is_disabled() {
        return true;
    }

    if !is_mcp_server_reachable() {
        return true;
    }

    if file_path.is_some_and(is_binary_file) {
        return true;
    }

    if is_replace_mode_disabled() {
        return true;
    }

    let _ = tool_name;
    false
}

fn is_mcp_server_reachable() -> bool {
    let path = crate::daemon::daemon_pid_path();
    if !path.exists() {
        return true;
    }
    if let Ok(pid_str) = std::fs::read_to_string(&path)
        && let Ok(pid) = pid_str.trim().parse::<u32>()
        && !crate::ipc::process::is_alive(pid)
    {
        return false;
    }
    true
}

fn is_replace_mode_disabled() -> bool {
    matches!(
        std::env::var("LEAN_CTX_REPLACE_MODE"),
        Ok(v) if v.trim() == "0" || v.trim().eq_ignore_ascii_case("off")
    )
}

fn is_binary_file(path: &str) -> bool {
    if let Some(ext) = path.rsplit('.').next() {
        return BINARY_EXTENSIONS.contains(&ext.to_lowercase().as_str());
    }
    false
}

fn extract_tool_name(payload: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) {
        if let Some(name) = json.get("tool_name").and_then(|v| v.as_str()) {
            return name.to_string();
        }
        if let Some(name) = json
            .get("hookSpecificInput")
            .and_then(|h| h.get("toolName"))
            .and_then(|v| v.as_str())
        {
            return name.to_string();
        }
    }
    "unknown".to_string()
}

fn extract_file_path(payload: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(payload).ok()?;

    let input = json
        .get("input")
        .or_else(|| json.get("hookSpecificInput").and_then(|h| h.get("input")));

    if let Some(input) = input {
        for key in ["file_path", "path", "filePath"] {
            if let Some(path) = input.get(key).and_then(|v| v.as_str()) {
                return Some(path.to_string());
            }
        }
    }
    None
}

fn print_deny(tool_name: &str) {
    let msg = match tool_name {
        "Read" | "read" | "ReadFile" => {
            "Use ctx_read instead — lean-ctx replace mode is active. Native Read is disabled."
        }
        "Grep" | "grep" | "Search" => {
            "Use ctx_search instead — lean-ctx replace mode is active. Native Grep is disabled."
        }
        "Glob" | "glob" => {
            "Use ctx_glob instead — lean-ctx replace mode is active. Native Glob is disabled."
        }
        "Shell" | "Bash" | "bash" => {
            "Use ctx_shell instead — lean-ctx replace mode is active. Native Shell is disabled."
        }
        _ => "Use the equivalent ctx_* tool — lean-ctx replace mode is active.",
    };

    let output = serde_json::json!({
        "permission": "deny",
        "user_message": msg
    });
    println!("{output}");
    std::process::exit(2);
}

fn print_allow() {
    println!("{{}}");
}

fn read_stdin_with_timeout() -> String {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = std::io::stdin().read_to_string(&mut buf);
        let _ = tx.send(buf);
    });
    rx.recv_timeout(HOOK_STDIN_TIMEOUT).unwrap_or_default()
}
