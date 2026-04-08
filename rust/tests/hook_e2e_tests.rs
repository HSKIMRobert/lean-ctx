//! End-to-end tests for the hook rewrite pipeline.
//!
//! These tests validate the ACTUAL JSON output produced by both:
//! 1. The Rust binary (`lean-ctx hook rewrite`)
//! 2. The generated Bash hook scripts
//!
//! Every test feeds real JSON to stdin and validates that stdout is
//! parseable JSON with the correct `hookSpecificOutput` structure.
//! This catches escaping bugs that unit tests on helper functions miss.

use std::io::Write;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Feed JSON to `lean-ctx hook rewrite` and return raw stdout (None = passthrough).
fn run_rust_rewrite(json_input: &str) -> Option<String> {
    let bin = env!("CARGO_BIN_EXE_lean-ctx");
    let mut child = Command::new(bin)
        .args(["hook", "rewrite"])
        .env("LEAN_CTX_DISABLED", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn lean-ctx");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(json_input.as_bytes())
        .unwrap();

    let output = child.wait_with_output().expect("failed to wait");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        None
    } else {
        Some(stdout)
    }
}

/// Feed JSON to a generated bash rewrite script and return raw stdout.
fn run_bash_rewrite(json_input: &str) -> Option<String> {
    let script = lean_ctx::hooks::generate_rewrite_script("lean-ctx");
    let script_path = std::path::PathBuf::from(format!(
        "/tmp/lean_ctx_test_{}_{}.sh",
        std::process::id(),
        std::thread::current()
            .name()
            .unwrap_or("t")
            .replace(' ', "_")
    ));
    std::fs::write(&script_path, &script).expect("write script");

    let mut child = Command::new("bash")
        .arg(&script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn bash");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(json_input.as_bytes())
        .unwrap();

    let output = child.wait_with_output().expect("failed to wait");
    let _ = std::fs::remove_file(&script_path);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        None
    } else {
        Some(stdout)
    }
}

/// Parse hook JSON output and extract the rewritten command string.
fn extract_command(raw_json: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(raw_json)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nraw: {raw_json}"));
    v["hookSpecificOutput"]["updatedInput"]["command"]
        .as_str()
        .unwrap_or_else(|| panic!("missing command field in: {raw_json}"))
        .to_string()
}

/// Build a JSON input string as Claude Code would send it.
fn bash_input(command: &str) -> String {
    let escaped = command.replace('\\', "\\\\").replace('"', "\\\"");
    format!(r#"{{"tool_name":"Bash","command":"{escaped}"}}"#)
}

/// Strip the binary path prefix from a rewritten command, leaving just `lean-ctx -c "..."`.
fn normalize_command(cmd: &str) -> String {
    if let Some(pos) = cmd.find("lean-ctx -c ") {
        cmd[pos..].to_string()
    } else {
        cmd.to_string()
    }
}

// ---------------------------------------------------------------------------
// Rust binary E2E tests
// ---------------------------------------------------------------------------

#[test]
fn rust_rewrite_simple_command() {
    let raw = run_rust_rewrite(&bash_input("git status")).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(cmd.ends_with(r#"-c "git status""#), "unexpected: {cmd}");
}

#[test]
fn rust_rewrite_pipe_command() {
    let raw = run_rust_rewrite(&bash_input("curl https://api.com | python3 -m json.tool"))
        .expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(
        cmd.contains("curl https://api.com | python3 -m json.tool"),
        "pipe must be preserved inside quotes: {cmd}"
    );
}

#[test]
fn rust_rewrite_embedded_quotes_git_commit() {
    let input = r#"{"tool_name":"Bash","command":"git commit --allow-empty -m \"Test\""}"#;
    let raw = run_rust_rewrite(input).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(
        cmd.contains(r#"-m \"Test\""#) || cmd.contains(r#"-m "Test""#),
        "quotes around Test must survive: {cmd}"
    );
}

#[test]
fn rust_rewrite_curl_with_auth_header() {
    let input = r#"{"tool_name":"Bash","command":"curl -H \"Authorization: Bearer token\" https://api.com"}"#;
    let raw = run_rust_rewrite(input).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(
        cmd.contains("Authorization: Bearer token"),
        "auth header must survive: {cmd}"
    );
}

#[test]
fn rust_rewrite_grep_with_quoted_pattern() {
    let input = r#"{"tool_name":"Bash","command":"grep -r \"TODO\" src/"}"#;
    let raw = run_rust_rewrite(input).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(cmd.contains("TODO"), "grep pattern must survive: {cmd}");
}

#[test]
fn rust_rewrite_docker_multiple_env() {
    let input = r#"{"tool_name":"Bash","command":"docker run -e \"A=1\" -e \"B=2\" nginx"}"#;
    let raw = run_rust_rewrite(input).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(cmd.contains("A=1"), "first env: {cmd}");
    assert!(cmd.contains("B=2"), "second env: {cmd}");
}

#[test]
fn rust_rewrite_find_glob() {
    let input = r#"{"tool_name":"Bash","command":"find . -name \"*.js\""}"#;
    let raw = run_rust_rewrite(input).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(cmd.contains("*.js"), "glob must survive: {cmd}");
}

#[test]
fn rust_rewrite_git_format() {
    let input = r#"{"tool_name":"Bash","command":"git log --format=\"%H %s\""}"#;
    let raw = run_rust_rewrite(input).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(cmd.contains("%H %s"), "format must survive: {cmd}");
}

#[test]
fn rust_passthrough_lean_ctx_self() {
    let input = r#"{"tool_name":"Bash","command":"lean-ctx read main.rs"}"#;
    assert!(
        run_rust_rewrite(input).is_none(),
        "lean-ctx commands must passthrough"
    );
}

#[test]
fn rust_passthrough_non_bash_tool() {
    let input = r#"{"tool_name":"Write","command":"test"}"#;
    assert!(
        run_rust_rewrite(input).is_none(),
        "non-Bash tools must passthrough"
    );
}

#[test]
fn rust_passthrough_echo() {
    let input = r#"{"tool_name":"Bash","command":"echo hello"}"#;
    assert!(
        run_rust_rewrite(input).is_none(),
        "echo is not in rewritable prefixes"
    );
}

// ---------------------------------------------------------------------------
// Bash script E2E tests
// ---------------------------------------------------------------------------

#[test]
fn bash_rewrite_simple_command() {
    let raw = run_bash_rewrite(&bash_input("git status")).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(cmd.contains("git status"), "unexpected: {cmd}");
}

#[test]
fn bash_rewrite_pipe_command() {
    let raw =
        run_bash_rewrite(&bash_input("curl https://api.com | python3")).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(
        cmd.contains("curl https://api.com | python3"),
        "pipe: {cmd}"
    );
}

#[test]
fn bash_rewrite_embedded_quotes_git_commit() {
    let input = r#"{"tool_name":"Bash","command":"git commit --allow-empty -m \"Test\""}"#;
    let raw = run_bash_rewrite(input).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(
        cmd.contains(r#"-m \"Test\""#) || cmd.contains(r#"-m "Test""#),
        "quotes around Test must survive: {cmd}"
    );
}

#[test]
fn bash_rewrite_curl_with_auth_header() {
    let input = r#"{"tool_name":"Bash","command":"curl -H \"Authorization: Bearer token\" https://api.com"}"#;
    let raw = run_bash_rewrite(input).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(
        cmd.contains("Authorization: Bearer token"),
        "auth header: {cmd}"
    );
}

#[test]
fn bash_rewrite_docker_multiple_env() {
    let input = r#"{"tool_name":"Bash","command":"docker run -e \"A=1\" -e \"B=2\" nginx"}"#;
    let raw = run_bash_rewrite(input).expect("should rewrite");
    let cmd = extract_command(&raw);
    assert!(cmd.contains("A=1"), "first env: {cmd}");
    assert!(cmd.contains("B=2"), "second env: {cmd}");
}

#[test]
fn bash_passthrough_lean_ctx_self() {
    let input = r#"{"tool_name":"Bash","command":"lean-ctx read main.rs"}"#;
    assert!(
        run_bash_rewrite(input).is_none(),
        "lean-ctx self must passthrough"
    );
}

#[test]
fn bash_passthrough_non_bash_tool() {
    let input = r#"{"tool_name":"Write","command":"test"}"#;
    assert!(
        run_bash_rewrite(input).is_none(),
        "non-Bash must passthrough"
    );
}

// ---------------------------------------------------------------------------
// Consistency: Rust binary vs. Bash script must produce identical commands
// ---------------------------------------------------------------------------

#[test]
fn consistency_simple() {
    let input = bash_input("git status");
    let rust_cmd = normalize_command(&extract_command(&run_rust_rewrite(&input).expect("rust")));
    let bash_cmd = normalize_command(&extract_command(&run_bash_rewrite(&input).expect("bash")));
    assert_eq!(rust_cmd, bash_cmd, "simple command mismatch");
}

#[test]
fn consistency_pipe() {
    let input = bash_input("git log --oneline | grep fix | head -5");
    let rust_cmd = normalize_command(&extract_command(&run_rust_rewrite(&input).expect("rust")));
    let bash_cmd = normalize_command(&extract_command(&run_bash_rewrite(&input).expect("bash")));
    assert_eq!(rust_cmd, bash_cmd, "pipe command mismatch");
}

#[test]
fn consistency_embedded_quotes() {
    let input = r#"{"tool_name":"Bash","command":"curl -H \"Auth\" https://api.com"}"#;
    let rust_cmd = normalize_command(&extract_command(&run_rust_rewrite(input).expect("rust")));
    let bash_cmd = normalize_command(&extract_command(&run_bash_rewrite(input).expect("bash")));
    assert_eq!(rust_cmd, bash_cmd, "embedded quotes mismatch");
}

#[test]
fn consistency_multiple_quotes() {
    let input = r#"{"tool_name":"Bash","command":"docker run -e \"A=1\" -e \"B=2\" nginx"}"#;
    let rust_cmd = normalize_command(&extract_command(&run_rust_rewrite(input).expect("rust")));
    let bash_cmd = normalize_command(&extract_command(&run_bash_rewrite(input).expect("bash")));
    assert_eq!(rust_cmd, bash_cmd, "multi-quote mismatch");
}

#[test]
fn consistency_npm() {
    let input = bash_input("npm run build");
    let rust_cmd = normalize_command(&extract_command(&run_rust_rewrite(&input).expect("rust")));
    let bash_cmd = normalize_command(&extract_command(&run_bash_rewrite(&input).expect("bash")));
    assert_eq!(rust_cmd, bash_cmd, "npm mismatch");
}
