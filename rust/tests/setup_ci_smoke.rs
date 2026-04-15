use std::process::Command;

use lean_ctx::core::setup_report::SetupReport;
use lean_ctx::status::StatusReport;
use lean_ctx::token_report::TokenReport;

fn run_json(bin: &str, args: &[&str], envs: &[(&str, &str)]) -> (i32, String) {
    let mut cmd = Command::new(bin);
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("process start");
    let code = out.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    (code, stdout)
}

#[test]
fn setup_bootstrap_doctor_status_json_smoke() {
    let bin = env!("CARGO_BIN_EXE_lean-ctx");

    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let data_dir = tmp.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    let home_str = home.to_string_lossy().to_string();
    let data_str = data_dir.to_string_lossy().to_string();

    let mut envs = vec![
        ("HOME", home_str.as_str()),
        ("LEAN_CTX_DATA_DIR", data_str.as_str()),
        ("LEAN_CTX_ACTIVE", "1"),
    ];

    #[cfg(not(windows))]
    {
        envs.push(("SHELL", "/bin/bash"));
    }
    #[cfg(windows)]
    {
        envs.push(("USERPROFILE", home_str.as_str()));
    }

    // bootstrap --json returns clean JSON (SetupReport)
    let (code, out) = run_json(bin, &["bootstrap", "--json"], &envs);
    assert_eq!(code, 0, "bootstrap exit code");
    let setup: SetupReport = serde_json::from_str(&out).expect("bootstrap JSON parse");
    assert_eq!(setup.schema_version, 1);

    // doctor --fix --json returns clean JSON (SetupReport shape)
    let (code, out) = run_json(bin, &["doctor", "--fix", "--json"], &envs);
    assert_eq!(code, 0, "doctor --fix exit code");
    let doctor_report: SetupReport = serde_json::from_str(&out).expect("doctor JSON parse");
    assert_eq!(doctor_report.schema_version, 1);

    // status --json returns clean JSON
    let (code, out) = run_json(bin, &["status", "--json"], &envs);
    assert_eq!(code, 0, "status exit code");
    let status: StatusReport = serde_json::from_str(&out).expect("status JSON parse");
    assert_eq!(status.schema_version, 1);

    // token-report --json returns clean JSON
    let (code, out) = run_json(bin, &["token-report", "--json"], &envs);
    assert_eq!(code, 0, "token-report exit code");
    let report: TokenReport = serde_json::from_str(&out).expect("token-report JSON parse");
    assert_eq!(report.schema_version, 1);
}
