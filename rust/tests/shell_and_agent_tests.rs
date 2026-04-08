//! E2E tests for shell detection, LEAN_CTX_SHELL override,
//! agent init (incl. antigravity alias), and Windows path handling.

use std::io::Write;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn lean_ctx_bin() -> String {
    env!("CARGO_BIN_EXE_lean-ctx").to_string()
}

fn run_with_env(
    args: &[&str],
    env_vars: &[(&str, &str)],
    stdin_data: Option<&str>,
) -> (String, String, i32) {
    let mut cmd = Command::new(lean_ctx_bin());
    cmd.args(args)
        .env("LEAN_CTX_DISABLED", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (k, v) in env_vars {
        cmd.env(k, v);
    }

    let mut child = cmd.spawn().expect("failed to spawn lean-ctx");

    if let Some(data) = stdin_data {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(data.as_bytes())
            .unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(1);
    (stdout, stderr, code)
}

// ---------------------------------------------------------------------------
// LEAN_CTX_SHELL override tests (via `lean-ctx -c`)
// ---------------------------------------------------------------------------

#[test]
fn lean_ctx_shell_override_uses_specified_shell() {
    let (stdout, _stderr, code) = run_with_env(
        &["-c", "echo lean_ctx_shell_works"],
        &[("LEAN_CTX_SHELL", "/bin/sh")],
        None,
    );
    assert_eq!(code, 0, "should succeed with /bin/sh");
    assert!(
        stdout.contains("lean_ctx_shell_works"),
        "should see echo output: {stdout}"
    );
}

#[test]
fn lean_ctx_shell_override_bash() {
    if !std::path::Path::new("/bin/bash").exists() {
        return;
    }
    let (stdout, _stderr, code) = run_with_env(
        &["-c", "echo $BASH_VERSION"],
        &[("LEAN_CTX_SHELL", "/bin/bash")],
        None,
    );
    assert_eq!(code, 0, "should succeed with /bin/bash");
    assert!(!stdout.trim().is_empty(), "BASH_VERSION should be set");
}

#[test]
fn lean_ctx_shell_override_invalid_shell_fails() {
    let (_stdout, _stderr, code) = run_with_env(
        &["-c", "echo hello"],
        &[("LEAN_CTX_SHELL", "/nonexistent/shell")],
        None,
    );
    assert_ne!(code, 0, "should fail with nonexistent shell");
}

// ---------------------------------------------------------------------------
// Shell command execution tests (basic sanity)
// ---------------------------------------------------------------------------

#[test]
fn shell_exec_simple_command() {
    let (stdout, _stderr, code) = run_with_env(&["-c", "echo hello_world"], &[], None);
    assert_eq!(code, 0);
    assert!(stdout.contains("hello_world"), "output: {stdout}");
}

#[test]
fn shell_exec_pipe_command() {
    let (stdout, _stderr, code) =
        run_with_env(&["-c", "echo 'line1\nline2\nline3' | head -1"], &[], None);
    assert_eq!(code, 0, "pipe should work");
    assert!(!stdout.trim().is_empty(), "should have output: {stdout}");
}

#[test]
fn shell_exec_and_chain() {
    let (stdout, _stderr, code) = run_with_env(&["-c", "echo first && echo second"], &[], None);
    assert_eq!(code, 0, "&& chain should work");
    assert!(stdout.contains("first"), "first: {stdout}");
    assert!(stdout.contains("second"), "second: {stdout}");
}

#[test]
fn shell_exec_semicolon_chain() {
    let (stdout, _stderr, code) = run_with_env(&["-c", "echo aaa; echo bbb"], &[], None);
    assert_eq!(code, 0, "; chain should work");
    assert!(stdout.contains("aaa"), "aaa: {stdout}");
    assert!(stdout.contains("bbb"), "bbb: {stdout}");
}

#[test]
fn shell_exec_subshell() {
    let (stdout, _stderr, code) = run_with_env(&["-c", "echo $(echo subshell_output)"], &[], None);
    assert_eq!(code, 0, "subshell should work");
    assert!(stdout.contains("subshell_output"), "subshell: {stdout}");
}

#[test]
fn shell_exec_env_var_expansion() {
    let (stdout, _stderr, code) = run_with_env(
        &["-c", "echo $TEST_LEAN_CTX_VAR"],
        &[("TEST_LEAN_CTX_VAR", "expanded_value")],
        None,
    );
    assert_eq!(code, 0);
    assert!(
        stdout.contains("expanded_value"),
        "env var expansion: {stdout}"
    );
}

#[test]
fn shell_exec_quoted_args() {
    let (stdout, _stderr, code) =
        run_with_env(&["-c", r#"echo "hello world with spaces""#], &[], None);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("hello world with spaces"),
        "quoted args: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// Agent init tests
// ---------------------------------------------------------------------------

#[test]
fn agent_init_antigravity_alias() {
    let tmpdir = tempfile::tempdir().expect("create tempdir");
    let home = tmpdir.path();

    let gemini_dir = home.join(".gemini");
    std::fs::create_dir_all(&gemini_dir).unwrap();

    let mut cmd = Command::new(lean_ctx_bin());
    cmd.args(["init", "--agent", "antigravity", "--global"])
        .env("HOME", home.to_str().unwrap())
        .env("LEAN_CTX_DISABLED", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output().expect("failed to run init");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("Unknown agent"),
        "antigravity should be recognized: {stderr}"
    );

    let hooks_dir = gemini_dir.join("hooks");
    if hooks_dir.exists() {
        let rewrite = hooks_dir.join("lean-ctx-rewrite-gemini.sh");
        assert!(rewrite.exists(), "rewrite script should be created");
        let content = std::fs::read_to_string(&rewrite).unwrap();
        assert!(
            content.contains("hookSpecificOutput"),
            "rewrite script should contain hook output format"
        );
    }
}

#[test]
fn agent_init_unknown_agent_fails() {
    let (_stdout, stderr, code) =
        run_with_env(&["init", "--agent", "nonexistent_agent"], &[], None);
    assert_ne!(code, 0, "unknown agent should fail");
    assert!(
        stderr.contains("Unknown agent"),
        "should say unknown: {stderr}"
    );
}

#[test]
fn agent_init_lists_antigravity_in_supported() {
    let (_stdout, stderr, _code) =
        run_with_env(&["init", "--agent", "nonexistent_agent"], &[], None);
    assert!(
        stderr.contains("antigravity"),
        "supported list should include antigravity: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Hook rewrite with LEAN_CTX_SHELL override
// ---------------------------------------------------------------------------

#[test]
fn hook_rewrite_works_with_shell_override() {
    let input = r#"{"tool_name":"Bash","command":"git status"}"#;
    let (stdout, _stderr, _code) = run_with_env(
        &["hook", "rewrite"],
        &[("LEAN_CTX_SHELL", "/bin/sh")],
        Some(input),
    );
    if !stdout.trim().is_empty() {
        let v: serde_json::Value =
            serde_json::from_str(&stdout).expect("hook output should be valid JSON");
        assert!(
            v["hookSpecificOutput"]["updatedInput"]["command"]
                .as_str()
                .is_some(),
            "should have command field"
        );
    }
}

// ---------------------------------------------------------------------------
// Windows path handling in generated scripts
// ---------------------------------------------------------------------------

#[test]
fn generated_script_handles_windows_path() {
    let script = lean_ctx::hooks::generate_rewrite_script("/c/Users/Jaina/bin/lean-ctx.exe");
    assert!(
        script.contains("LEAN_CTX_BIN=\"/c/Users/Jaina/bin/lean-ctx.exe\""),
        "Windows bash path should be properly quoted in script"
    );
}

#[test]
fn generated_script_handles_path_with_spaces() {
    let script = lean_ctx::hooks::generate_rewrite_script("/c/Program Files/lean-ctx/lean-ctx.exe");
    assert!(
        script.contains("LEAN_CTX_BIN=\"/c/Program Files/lean-ctx/lean-ctx.exe\""),
        "path with spaces should be quoted"
    );
}

#[test]
fn generated_compact_script_handles_windows_path() {
    let script =
        lean_ctx::hooks::generate_compact_rewrite_script("/c/Users/Jaina/bin/lean-ctx.exe");
    assert!(
        script.contains("LEAN_CTX_BIN=\"/c/Users/Jaina/bin/lean-ctx.exe\""),
        "compact script should handle Windows path"
    );
}

#[test]
fn generated_script_skips_own_binary() {
    let script = lean_ctx::hooks::generate_rewrite_script("lean-ctx");
    assert!(
        script.contains("lean-ctx ") || script.contains("$LEAN_CTX_BIN "),
        "script should reference lean-ctx for self-skip check"
    );
}

// ---------------------------------------------------------------------------
// Bash script execution with Windows-style binary path
// ---------------------------------------------------------------------------

#[test]
fn bash_script_with_windows_binary_path_produces_valid_json() {
    let script =
        lean_ctx::hooks::generate_compact_rewrite_script("/c/Users/Jaina/bin/lean-ctx.exe");
    let script_path = std::path::PathBuf::from(format!(
        "/tmp/lean_ctx_winpath_test_{}.sh",
        std::process::id()
    ));
    std::fs::write(&script_path, &script).expect("write script");

    let input = r#"{"tool_name":"Bash","command":"git status"}"#;
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
        .write_all(input.as_bytes())
        .unwrap();

    let output = child.wait_with_output().expect("failed to wait");
    let _ = std::fs::remove_file(&script_path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!("invalid JSON from Windows path script: {e}\nraw: {stdout}")
        });
        let cmd = v["hookSpecificOutput"]["updatedInput"]["command"]
            .as_str()
            .expect("should have command");
        assert!(
            cmd.contains("/c/Users/Jaina/bin/lean-ctx.exe"),
            "rewritten command should use the Windows bash path: {cmd}"
        );
        assert!(
            cmd.contains("git status"),
            "original command should be preserved: {cmd}"
        );
    }
}
