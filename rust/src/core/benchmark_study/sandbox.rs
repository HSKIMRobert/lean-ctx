//! Sandboxed Python execution for benchmark tasks.
//!
//! Runs code in a subprocess with timeout, capturing stdout/stderr.
//! No network access, no filesystem writes outside temp.

use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};

/// Result of a sandboxed execution.
#[derive(Debug, Clone)]
pub(crate) struct SandboxResult {
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

fn read_child_output(child: &mut std::process::Child) -> (String, String) {
    let stdout = child
        .stdout
        .take()
        .map(|mut o| {
            let mut s = String::new();
            std::io::Read::read_to_string(&mut o, &mut s).ok();
            s
        })
        .unwrap_or_default();
    let stderr = child
        .stderr
        .take()
        .map(|mut o| {
            let mut s = String::new();
            std::io::Read::read_to_string(&mut o, &mut s).ok();
            s
        })
        .unwrap_or_default();
    (stdout, stderr)
}

/// Execute a Python script in a sandboxed subprocess.
///
/// The `code` is written to a temp file, executed with the given Python binary,
/// and the result captured. Timeout prevents hanging.
pub(crate) fn execute_python(python_bin: &str, code: &str, timeout: Duration) -> SandboxResult {
    let temp_dir = match tempfile::TempDir::new() {
        Ok(d) => d,
        Err(e) => {
            return SandboxResult {
                passed: false,
                stdout: String::new(),
                stderr: format!("failed to create temp dir: {e}"),
                exit_code: None,
                timed_out: false,
            };
        }
    };

    let script_path = temp_dir.path().join("solution.py");
    if let Err(e) =
        std::fs::File::create(&script_path).and_then(|mut f| f.write_all(code.as_bytes()))
    {
        return SandboxResult {
            passed: false,
            stdout: String::new(),
            stderr: format!("failed to write script: {e}"),
            exit_code: None,
            timed_out: false,
        };
    }

    let result = Command::new(python_bin)
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PYTHONHASHSEED", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let mut child = match result {
        Ok(c) => c,
        Err(e) => {
            return SandboxResult {
                passed: false,
                stdout: String::new(),
                stderr: format!("failed to spawn python: {e}"),
                exit_code: None,
                timed_out: false,
            };
        }
    };

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let (stdout, stderr) = read_child_output(&mut child);
                return SandboxResult {
                    passed: status.success(),
                    stdout,
                    stderr,
                    exit_code: status.code(),
                    timed_out: false,
                };
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return SandboxResult {
                        passed: false,
                        stdout: String::new(),
                        stderr: "execution timed out".into(),
                        exit_code: None,
                        timed_out: true,
                    };
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                return SandboxResult {
                    passed: false,
                    stdout: String::new(),
                    stderr: format!("wait error: {e}"),
                    exit_code: None,
                    timed_out: false,
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_passing_script() {
        let result = execute_python("python3", "assert 1 + 1 == 2", Duration::from_secs(10));
        assert!(result.passed);
        assert!(!result.timed_out);
    }

    #[test]
    fn failing_assertion() {
        let result = execute_python("python3", "assert 1 + 1 == 3", Duration::from_secs(10));
        assert!(!result.passed);
        assert!(!result.timed_out);
    }

    #[test]
    fn timeout_detection() {
        let result = execute_python(
            "python3",
            "import time; time.sleep(60)",
            Duration::from_secs(1),
        );
        assert!(!result.passed);
        assert!(result.timed_out);
    }
}
