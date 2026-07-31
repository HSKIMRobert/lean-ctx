//! Deterministic, read-only shell commands eligible for cross-agent caching.

use super::cache_types::CacheKey;
use dashmap::DashMap;
use std::path::Path;
use std::sync::LazyLock;

/// Process-local result cache for deterministic shell commands.
pub static SHELL_RESULT_CACHE: LazyLock<DashMap<CacheKey, String>> = LazyLock::new(DashMap::new);

/// Commands whose output is deterministic for an unchanged workspace state.
pub static CACHEABLE_COMMANDS: &[&str] = &["cargo", "rg", "grep", "wc", "ls", "find", "du", "git"];

/// Returns whether `command` is one read-only command with deterministic output.
pub fn is_cacheable_command(command: &str) -> bool {
    let tokens = crate::core::shell_allowlist::shell_tokenize(command.trim());
    let Some(program) = tokens.first().map(String::as_str) else {
        return false;
    };
    if !CACHEABLE_COMMANDS.contains(&program) {
        return false;
    }
    if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "|" | "&&" | "||" | ";" | ">" | ">>" | "<"))
    {
        return false;
    }
    match program {
        "cargo" => tokens.get(1).is_some_and(|subcommand| subcommand == "test"),
        "rg" | "grep" | "wc" | "ls" | "find" | "du" => true,
        "git" => matches!(
            tokens.get(1).map(String::as_str),
            Some("log" | "status" | "diff")
        ),
        _ => false,
    }
}

/// Normalizes whitespace, option order, and local absolute paths for cache keys.
pub fn normalize_command(command: &str) -> String {
    let mut tokens = crate::core::shell_allowlist::shell_tokenize(command.trim());
    if tokens.is_empty() {
        return String::new();
    }

    let root = std::env::current_dir().ok();
    for token in &mut tokens {
        *token = normalize_path_token(token, root.as_deref());
    }

    // Flags are independent for the supported read-only commands. Keep every
    // positional argument in its original location, but canonicalize a leading
    // run of flags (the conventional command-line shape).
    let first_positional = tokens[1..]
        .iter()
        .position(|token| !token.starts_with('-'))
        .map_or(tokens.len(), |offset| offset + 1);
    tokens[1..first_positional].sort();
    tokens.join(" ")
}

fn normalize_path_token(token: &str, root: Option<&Path>) -> String {
    if !token.starts_with('/') {
        return token.to_string();
    }
    let path = Path::new(token);
    if let Some(root) = root
        && let Ok(relative) = path.strip_prefix(root)
    {
        return if relative.as_os_str().is_empty() {
            "$PROJECT_ROOT".to_string()
        } else {
            format!("$PROJECT_ROOT/{}", relative.to_string_lossy())
        };
    }
    "$PROJECT_ROOT".to_string()
}

#[cfg(test)]
mod tests {
    use super::{is_cacheable_command, normalize_command};

    #[test]
    fn allowlist_accepts_the_supported_read_only_commands() {
        for command in [
            "cargo test",
            "cargo test --lib",
            "rg needle src",
            "grep -R needle src",
            "wc -l src/lib.rs",
            "ls -la",
            "find src -name '*.rs'",
            "du -sh target",
            "git log --oneline",
            "git status --short",
            "git diff --stat",
        ] {
            assert!(is_cacheable_command(command), "{command}");
        }
    }

    #[test]
    fn allowlist_rejects_mutation_and_shell_composition() {
        for command in [
            "cargo build",
            "git commit -m x",
            "rg needle | wc -l",
            "echo x",
        ] {
            assert!(!is_cacheable_command(command), "{command}");
        }
    }

    #[test]
    fn normalization_sorts_leading_flags_and_removes_absolute_paths() {
        assert_eq!(normalize_command(" rg   -n -i needle  "), "rg -i -n needle");
        assert_eq!(
            normalize_command("rg needle /tmp/other"),
            "rg needle $PROJECT_ROOT"
        );
    }
}
