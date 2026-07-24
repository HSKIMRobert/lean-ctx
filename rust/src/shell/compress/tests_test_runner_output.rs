use super::*;
/// Builds a realistic large, fully-passing multi-suite cargo test output:
/// no "error"/"FAILED" words anywhere, so it exercises the all-green path.
fn large_passing_cargo_test_output(suites: usize) -> String {
    let mut out = String::new();
    for s in 0..suites {
        out.push_str(&format!(
            "     Running tests/suite_{s}.rs (target/debug/deps/suite_{s}-abc)\n\n"
        ));
        out.push_str("running 3 tests\n");
        out.push_str(&format!("test suite_{s}::alpha ... ok\n"));
        out.push_str(&format!("test suite_{s}::beta ... ok\n"));
        out.push_str(&format!("test suite_{s}::gamma ... ok\n\n"));
        out.push_str(&format!(
                "test result: ok. {} passed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0{s}s\n\n",
                3 + s
            ));
    }
    out
}

#[test]
fn cargo_test_keeps_every_result_line_when_large() {
    let output = large_passing_cargo_test_output(60);
    let result = compress_if_beneficial("cargo test --all-features", &output);
    // The unique passed-count of every suite must survive.
    for s in 0..60 {
        let needle = format!("{} passed", 3 + s);
        assert!(
            result.contains(&needle),
            "suite {s} summary '{needle}' was lost during compression"
        );
    }
}

#[test]
fn piped_cargo_test_keeps_result_lines() {
    // `cargo test … | grep … | tail` — the pipeline form that originally
    // lost its result lines. Each segment is checked for a test runner.
    let mut grepped = String::new();
    for s in 0..60 {
        grepped.push_str(&format!(
            "test result: ok. {} passed; 0 failed; 0 ignored; finished in 0.0{s}s\n",
            3 + s
        ));
    }
    let result = compress_if_beneficial(
        "cargo test --all-features 2>&1 | grep -E 'test result:' | tail -100",
        &grepped,
    );
    for s in 0..60 {
        let needle = format!("{} passed", 3 + s);
        assert!(
            result.contains(&needle),
            "piped suite {s} summary '{needle}' was lost"
        );
    }
}

#[test]
fn pytest_summary_survives() {
    let mut output = String::from("============ test session starts ============\n");
    for i in 0..400 {
        output.push_str(&format!("tests/test_mod.py::test_case_{i} PASSED\n"));
    }
    output.push_str("\n============ 400 passed in 12.34s ============\n");
    let result = compress_if_beneficial("pytest -q", &output);
    assert!(
        result.contains("400 passed in 12.34s"),
        "pytest summary line must survive, got tail: {}",
        &result[result.len().saturating_sub(200)..]
    );
}

#[test]
fn env_prefixed_test_command_is_recognized() {
    // `RUST_BACKTRACE=1 cargo test` must still be treated as a test runner.
    let output = large_passing_cargo_test_output(60);
    let result = compress_if_beneficial("RUST_BACKTRACE=1 cargo test --workspace", &output);
    assert!(
        result.contains("62 passed"),
        "env-prefixed cargo test lost its last suite summary"
    );
    // And the same for pytest with a CI env prefix.
    let mut py = String::from("============ test session starts ============\n");
    for i in 0..400 {
        py.push_str(&format!("tests/test_mod.py::test_case_{i} PASSED\n"));
    }
    py.push_str("\n============ 400 passed in 9.99s ============\n");
    let py_result = compress_if_beneficial("CI=true python3 -m pytest -q", &py);
    assert!(
        py_result.contains("400 passed in 9.99s"),
        "env-prefixed pytest lost its summary"
    );
}

#[test]
fn buried_failure_in_large_passing_run_survives() {
    // A single FAILED line buried deep in an otherwise-passing run must not
    // be truncated away.
    let mut output = String::new();
    for i in 0..500 {
        output.push_str(&format!("test mod::case_{i} ... ok\n"));
    }
    output.insert_str(
        output.len() / 2,
        "test mod::critical_case ... FAILED\n\nfailures:\n    mod::critical_case\n",
    );
    let result = compress_if_beneficial("cargo test", &output);
    assert!(
        result.contains("critical_case ... FAILED"),
        "a buried FAILED line must never be dropped"
    );
}
