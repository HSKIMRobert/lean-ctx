use std::process::Command;

fn lean_ctx_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_lean-ctx"));
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    cmd.env("LEAN_CTX_ACTIVE", "1");
    cmd
}

#[test]
fn binary_prints_version() {
    let output = lean_ctx_bin()
        .arg("--version")
        .output()
        .expect("failed to run lean-ctx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("lean-ctx"),
        "version output should contain 'lean-ctx', got: {stdout}"
    );
}

#[test]
fn binary_prints_help() {
    let output = lean_ctx_bin()
        .arg("--help")
        .output()
        .expect("failed to run lean-ctx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Intelligence Layer"),
        "help should contain tagline"
    );
    assert!(stdout.contains("lean-ctx"), "help should mention lean-ctx");
}

#[test]
fn binary_read_file() {
    let output = lean_ctx_bin()
        .args(["read", "Cargo.toml", "-m", "signatures"])
        .output()
        .expect("failed to run lean-ctx");
    assert!(output.status.success(), "read should succeed");
}

#[test]
fn binary_config_shows_defaults() {
    let output = lean_ctx_bin()
        .arg("config")
        .output()
        .expect("failed to run lean-ctx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("checkpoint_interval"),
        "config should show checkpoint_interval"
    );
}

#[test]
fn shell_hook_compresses_echo() {
    let output = lean_ctx_bin()
        .args(["-c", "echo", "hello", "world"])
        .output()
        .expect("failed to run lean-ctx -c");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello"),
        "shell hook should pass through echo output"
    );
}

#[test]
fn disabled_env_bypasses_compression() {
    let output = Command::new(env!("CARGO_BIN_EXE_lean-ctx"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("LEAN_CTX_DISABLED", "1")
        .env("LEAN_CTX_COMPRESS", "1")
        .args(["-c", "echo", "passthrough test"])
        .output()
        .expect("failed to run lean-ctx with LEAN_CTX_DISABLED");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("passthrough"),
        "LEAN_CTX_DISABLED should pass output through unmodified"
    );
    assert!(
        !stdout.contains("[lean-ctx:"),
        "LEAN_CTX_DISABLED should not add compression markers"
    );
}

#[test]
fn help_shows_environment_section() {
    let output = lean_ctx_bin()
        .arg("--help")
        .output()
        .expect("failed to run lean-ctx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("LEAN_CTX_DISABLED"),
        "help should document LEAN_CTX_DISABLED"
    );
    assert!(
        stdout.contains("LEAN_CTX_RAW"),
        "help should document LEAN_CTX_RAW"
    );
}
