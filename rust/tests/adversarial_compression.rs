//! Adversarial compression tests based on TheDecipherist/rtk-test methodology.
//! Each test verifies that safety-critical information survives compression.

use lean_ctx::core::patterns::compress_output;

#[test]
fn adversarial_git_diff_preserves_code_content() {
    let diff = "diff --git a/src/auth.rs b/src/auth.rs\n\
                index abc123..def456 100644\n\
                --- a/src/auth.rs\n\
                +++ b/src/auth.rs\n\
                @@ -10,6 +10,8 @@ fn verify_token(token: &str) -> bool {\n\
                     let decoded = decode(token);\n\
                     if decoded.is_err() {\n\
                         return false;\n\
                +    }\n\
                +    if decoded.unwrap().exp < now() {\n\
                +        return false; // expired token\n\
                     }\n\
                     true\n\
                 }";

    let compressed = compress_output("git diff", diff).unwrap();
    assert!(
        compressed.contains("expired token"),
        "diff must preserve code content: {compressed}"
    );
    assert!(
        compressed.contains("+"),
        "diff must preserve +/- markers: {compressed}"
    );
}

#[test]
fn adversarial_git_diff_preserves_security_bug() {
    let diff = "diff --git a/src/api.rs b/src/api.rs\n\
                --- a/src/api.rs\n\
                +++ b/src/api.rs\n\
                @@ -5,3 +5,5 @@\n\
                -    verify_csrf_token(&request);\n\
                +    // TODO: re-enable CSRF check\n\
                +    // verify_csrf_token(&request);\n\
                     process_request(&request);\n";

    let compressed = compress_output("git diff", diff).unwrap();
    assert!(
        compressed.contains("CSRF") || compressed.contains("csrf"),
        "diff must preserve security-relevant changes: {compressed}"
    );
    assert!(
        compressed.contains("verify_csrf_token"),
        "diff must preserve removed function calls: {compressed}"
    );
}

#[test]
fn adversarial_docker_ps_preserves_unhealthy() {
    let ps_output = "CONTAINER ID   IMAGE          COMMAND       CREATED       STATUS                     PORTS     NAMES\n\
                     abc123def456   nginx:latest   \"nginx -g…\"   2 hours ago   Up 2 hours (unhealthy)     80/tcp    web-prod\n\
                     789ghi012jkl   redis:7        \"redis-se…\"   3 hours ago   Up 3 hours (healthy)       6379/tcp  cache-prod\n\
                     345mno678pqr   postgres:16    \"docker-e…\"   5 hours ago   Exited (1) 30 minutes ago            db-prod";

    let compressed = compress_output("docker ps", ps_output).unwrap();
    assert!(
        compressed.contains("unhealthy"),
        "docker ps must preserve unhealthy status: {compressed}"
    );
    assert!(
        compressed.contains("Exited"),
        "docker ps must preserve Exited status: {compressed}"
    );
    assert!(
        compressed.contains("web-prod"),
        "docker ps must preserve container names: {compressed}"
    );
}

#[test]
fn adversarial_df_preserves_root_filesystem() {
    let mut lines = vec!["Filesystem     1K-blocks    Used Available Use% Mounted on".to_string()];
    lines.push("/dev/sda1  100000000  95000000  5000000  95% /".to_string());
    for i in 0..15 {
        lines.push(format!(
            "tmpfs             1000      100      900   10% /snap/core/{i}"
        ));
    }
    let df_output = lines.join("\n");

    let compressed = compress_output("df -h", &df_output).unwrap();
    assert!(
        compressed.contains("/dev/sda1") || compressed.contains("95%"),
        "df must preserve root filesystem info: {compressed}"
    );
    assert!(
        compressed.contains("/ ") || compressed.contains("Mounted on"),
        "df must preserve mount points: {compressed}"
    );
}

#[test]
fn adversarial_pytest_preserves_xfail_xpass() {
    let output = "============================= test session starts ==============================\n\
                  collected 20 items\n\
                  \n\
                  tests/test_auth.py ....x.X...                                          [100%]\n\
                  \n\
                  ================== 15 passed, 2 xfailed, 1 xpassed, 2 warnings in 3.5s ==================";

    let compressed = compress_output("pytest", output).unwrap();
    assert!(
        compressed.contains("xfailed") || compressed.contains("xfail"),
        "pytest must preserve xfailed counter: {compressed}"
    );
    assert!(
        compressed.contains("xpassed") || compressed.contains("xpass"),
        "pytest must preserve xpassed counter: {compressed}"
    );
    assert!(
        compressed.contains("warning"),
        "pytest must preserve warnings counter: {compressed}"
    );
}

#[test]
fn adversarial_git_log_preserves_full_history() {
    let mut log_lines: Vec<String> = Vec::new();
    for i in 0..60 {
        log_lines.push(format!("abc{i:04x} fix: commit message number {i}"));
    }
    let output = log_lines.join("\n");

    let compressed_unlimited = compress_output("git log -n 60 --oneline", &output).unwrap();
    assert!(
        compressed_unlimited.contains("commit message number 59"),
        "git log with explicit -n must preserve all entries: {compressed_unlimited}"
    );

    let compressed_default = compress_output("git log --oneline", &output).unwrap();
    assert!(
        compressed_default.contains("commit message number 49")
            || compressed_default.contains("50"),
        "git log default should show at least 50 entries: {compressed_default}"
    );
}

#[test]
fn adversarial_grep_preserves_context() {
    let mut grep_lines: Vec<String> = Vec::new();
    for i in 0..80 {
        grep_lines.push(format!("src/auth.rs:{i}:    let user = get_user(id);"));
    }
    let output = grep_lines.join("\n");

    let compressed = compress_output("grep -rn 'get_user'", &output).unwrap();
    assert!(
        compressed.contains("get_user"),
        "grep output <=100 lines must pass through verbatim: {compressed}"
    );
    assert_eq!(
        compressed
            .lines()
            .filter(|l| l.contains("get_user"))
            .count(),
        80,
        "all 80 grep matches must be preserved: {compressed}"
    );
}

#[test]
fn adversarial_log_preserves_critical_severity() {
    let mut log = Vec::new();
    for i in 0..40 {
        log.push(format!("2024-01-01 10:00:{i:02} INFO  request processed"));
    }
    log.insert(
        20,
        "2024-01-01 10:00:20 CRITICAL database connection lost".to_string(),
    );
    log.insert(
        25,
        "2024-01-01 10:00:25 ERROR OOMKilled: container exceeded memory".to_string(),
    );
    let output = log.join("\n");

    let compressed = compress_output("cat /var/log/app.log", &output);
    let text = compressed.unwrap_or_else(|| output.clone());
    assert!(
        text.contains("CRITICAL") || text.contains("database connection lost"),
        "log output must preserve CRITICAL lines: {text}"
    );
}

#[test]
fn adversarial_npm_audit_preserves_cve_ids() {
    let audit = "# npm audit report\n\
                 \n\
                 lodash  <=4.17.20\n\
                 Severity: critical\n\
                 Prototype Pollution - https://github.com/advisories/GHSA-xxxx\n\
                 fix available via `npm audit fix --force`\n\
                 depends on vulnerable versions of lodash\n\
                 node_modules/lodash\n\
                 \n\
                 express  <4.17.3\n\
                 Severity: high\n\
                 CVE-2024-12345 - Open redirect vulnerability\n\
                 fix available via `npm audit fix`\n\
                 node_modules/express\n\
                 \n\
                 2 vulnerabilities (1 high, 1 critical)\n";

    let compressed = compress_output("npm audit", audit).unwrap();
    assert!(
        compressed.contains("CVE-2024-12345"),
        "npm audit must preserve CVE IDs: {compressed}"
    );
    assert!(
        compressed.contains("critical"),
        "npm audit must preserve severity levels: {compressed}"
    );
}

#[test]
fn adversarial_docker_logs_preserves_critical() {
    let mut log = Vec::new();
    for i in 0..50 {
        log.push(format!(
            "2024-01-01T10:00:{i:02}Z INFO  healthy check passed"
        ));
    }
    log.insert(
        15,
        "2024-01-01T10:00:15Z FATAL  out of memory, container killed".to_string(),
    );
    log.insert(
        30,
        "2024-01-01T10:00:30Z ERROR  panic: runtime error".to_string(),
    );
    let output = log.join("\n");

    let compressed = compress_output("docker logs mycontainer", &output).unwrap();
    assert!(
        compressed.contains("FATAL") || compressed.contains("out of memory"),
        "docker logs must preserve FATAL lines: {compressed}"
    );
}

#[test]
fn adversarial_pip_uninstall_preserves_package_names() {
    let output = "Found existing installation: requests 2.28.0\n\
                  Uninstalling requests-2.28.0:\n\
                    Successfully uninstalled requests-2.28.0\n\
                  Found existing installation: flask 2.3.0\n\
                  Uninstalling flask-2.3.0:\n\
                    Successfully uninstalled flask-2.3.0\n\
                  Found existing installation: numpy 1.24.0\n\
                  Uninstalling numpy-1.24.0:\n\
                    Successfully uninstalled numpy-1.24.0\n";

    let compressed = compress_output("pip uninstall requests flask numpy -y", output).unwrap();
    assert!(
        compressed.contains("requests"),
        "pip uninstall must list package names: {compressed}"
    );
    assert!(
        compressed.contains("flask"),
        "pip uninstall must list package names: {compressed}"
    );
    assert!(
        compressed.contains("numpy"),
        "pip uninstall must list package names: {compressed}"
    );
}

#[test]
fn adversarial_middle_truncation_preserves_errors() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..60 {
        lines.push(format!("line {i}: normal output"));
    }
    lines[30] = "ERROR: critical failure in module X".to_string();
    lines[35] = "WARNING: disk space low".to_string();
    let output = lines.join("\n");

    let compressed = lean_ctx::shell::compress_if_beneficial_pub("unknown-command", &output);
    if compressed.contains("[") && compressed.contains("omitted") {
        assert!(
            compressed.contains("ERROR") || compressed.contains("critical failure"),
            "truncation must preserve error lines: {compressed}"
        );
    }
}

// ===== Regression tests: Scenarios that were SAFE in TheDecipherist/rtk-test v3.2.5 =====
// These must stay SAFE after the adversarial hardening changes.

#[test]
fn regression_git_status_detached_head() {
    let output = "HEAD detached at 48a7098\nnothing to commit, working tree clean";
    let compressed = compress_output("git status", output).unwrap();
    assert!(
        compressed.contains("detached") || compressed.contains("HEAD detached"),
        "git status must preserve DETACHED HEAD warning: {compressed}"
    );
}

#[test]
fn regression_log_critical_severity() {
    let output = "[INFO] health check ok\n\
                  [INFO] health check ok\n\
                  [CRITICAL] database connection lost\n\
                  [INFO] health check ok\n\
                  [ERROR] retry failed";
    let compressed = compress_output("cat /var/log/app.log", output);
    let text = compressed.unwrap_or_else(|| output.to_string());
    assert!(
        text.contains("CRITICAL"),
        "cat log must preserve CRITICAL lines: {text}"
    );
    assert!(
        text.contains("ERROR"),
        "cat log must preserve ERROR lines: {text}"
    );
}

#[test]
fn regression_ls_shows_dotenv() {
    let output = ".env\n.gitignore\nREADME.md\nsrc\npackage.json";
    let compressed = compress_output("ls -a", output);
    let text = compressed.unwrap_or_else(|| output.to_string());
    assert!(text.contains(".env"), "ls must show .env file: {text}");
}

#[test]
fn regression_pip_list_all_packages() {
    let mut lines = vec![
        "Package    Version".to_string(),
        "---------- -------".to_string(),
    ];
    for i in 0..50 {
        lines.push(format!("package-{i}  1.0.{i}"));
    }
    let output = lines.join("\n");
    let compressed = compress_output("pip list", &output);
    let text = compressed.unwrap_or_else(|| output.to_string());
    assert!(
        text.contains("package-0") && text.contains("package-49"),
        "pip list must show all packages: first and last must be present"
    );
}

#[test]
fn regression_git_stash_verbatim() {
    let output = "No local changes to save";
    let compressed = compress_output("git stash", output);
    let text = compressed.unwrap_or_else(|| output.to_string());
    assert!(
        text.contains("No local changes"),
        "git stash must pass through verbatim: {text}"
    );

    let output2 = "Saved working directory and index state WIP on main: abc1234 fix: typo";
    let compressed2 = compress_output("git stash", output2);
    let text2 = compressed2.unwrap_or_else(|| output2.to_string());
    assert!(
        text2.contains("Saved") || text2.contains("WIP on main"),
        "git stash save must pass through: {text2}"
    );
}

#[test]
fn regression_ruff_preserves_file_line_col() {
    let output = "src/api.py:42:10: E501 Line too long (120 > 79)\n\
                  src/api.py:88:1: F401 'os' imported but unused\n\
                  Found 2 errors.";
    let compressed = compress_output("ruff check", output);
    let text = compressed.unwrap_or_else(|| output.to_string());
    assert!(
        text.contains("src/api.py:42:10"),
        "ruff must preserve file:line:col references: {text}"
    );
    assert!(
        text.contains("src/api.py:88:1"),
        "ruff must preserve all references: {text}"
    );
}

#[test]
fn regression_find_preserves_full_paths() {
    let output = "/home/user/project/src/api/file.ts\n\
                  /home/user/project/src/utils/helper.ts\n\
                  /home/user/project/tests/test_api.ts";
    let compressed = compress_output("find . -name '*.ts'", output);
    let text = compressed.unwrap_or_else(|| output.to_string());
    assert!(
        text.contains("/home/user/project/src/api/file.ts"),
        "find must preserve full absolute paths: {text}"
    );
}

#[test]
fn regression_ls_recursive_preserves_tree() {
    let output = "./src:\napi.ts\nutils.ts\n\n./src/components:\nButton.tsx\nHeader.tsx\n\n./tests:\ntest_api.ts";
    let compressed = compress_output("ls -R", output);
    let text = compressed.unwrap_or_else(|| output.to_string());
    assert!(
        text.contains("./src:") && text.contains("./tests:"),
        "ls -R must preserve directory headers: {text}"
    );
}

#[test]
fn regression_wc_pipe_correct() {
    let output = "42";
    let compressed = compress_output("wc -l", output);
    let text = compressed.unwrap_or_else(|| output.to_string());
    assert!(text.contains("42"), "wc output must be preserved: {text}");
}
