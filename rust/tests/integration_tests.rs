use std::process::Command;

#[test]
fn binary_prints_version() {
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "--version"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
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
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run lean-ctx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Cognitive Filter"),
        "help should contain tagline"
    );
    assert!(stdout.contains("MCP"), "help should mention MCP");
}

#[test]
fn binary_doctor_runs() {
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "doctor"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run lean-ctx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("lean-ctx") || stdout.contains("checks"),
        "doctor should produce diagnostic output"
    );
}

#[test]
fn binary_read_file() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "read",
            "Cargo.toml",
            "-m",
            "signatures",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run lean-ctx");
    assert!(output.status.success(), "read should succeed");
}

#[test]
fn binary_config_shows_defaults() {
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "config"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
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
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "-c", "echo", "hello", "world"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run lean-ctx -c");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello"),
        "shell hook should pass through echo output"
    );
}
