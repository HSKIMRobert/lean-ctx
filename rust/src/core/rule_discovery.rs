//! Rule discovery for ctx_read (#1325).
//!
//! When lean-ctx replaces native file-read tools, the IDE's rule-injection
//! contract must be honoured: reading a file should surface rules scoped to
//! that path (CLAUDE.md, .claude/rules, .cursor/rules, AGENTS.md).
//!
//! This module discovers applicable rules for a given file path, caches them
//! per directory, and formats them for appending to `ctx_read` output.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DiscoveredRule {
    pub source: String,
    pub content: String,
}

// ── Per-session dedup ───────────────────────────────────────────────────────

static INJECTED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn already_injected(key: &str) -> bool {
    let mut guard = INJECTED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let set = guard.get_or_insert_with(HashSet::new);
    !set.insert(key.to_string())
}

/// Reset injection tracking (for tests).
#[cfg(test)]
pub fn reset_injection_cache() {
    let mut guard = INJECTED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = None;
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Discover and format rules applicable to `file_path` that haven't been
/// injected yet in this session. Returns an empty string when no new rules
/// apply or when the client natively handles rule injection.
pub fn rules_suffix_for_read(file_path: &str, project_root: &str, client_id: &str) -> String {
    if client_natively_injects_rules(client_id) {
        return String::new();
    }

    let rules = discover_rules(file_path, project_root, client_id);
    if rules.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();
    for rule in &rules {
        let key = format!("{}:{}", rule.source, blake3_short(&rule.content));
        if already_injected(&key) {
            continue;
        }
        parts.push(format!("[From: {}]\n{}", rule.source, rule.content.trim()));
    }

    if parts.is_empty() {
        return String::new();
    }

    format!(
        "\n\n--- Rules in scope for this file ---\n{}",
        parts.join("\n\n")
    )
}

// ── Client detection ────────────────────────────────────────────────────────

/// Returns true when the client's native harness already injects rules on
/// file reads, so lean-ctx should NOT duplicate them.
///
/// Currently no client does this natively when lean-ctx intercepts the read
/// (the whole point of #1325), but this gate lets us skip injection if a
/// future client version adds native support.
fn client_natively_injects_rules(_client_id: &str) -> bool {
    false
}

// ── Discovery ───────────────────────────────────────────────────────────────

static DIR_CACHE: Mutex<Option<HashMap<String, Vec<DiscoveredRule>>>> = Mutex::new(None);

fn discover_rules(file_path: &str, project_root: &str, client_id: &str) -> Vec<DiscoveredRule> {
    let file = Path::new(file_path);
    let root = Path::new(project_root);
    let dir = file.parent().unwrap_or(root);

    let cache_key = format!("{client_id}:{}", dir.display());
    {
        let guard = DIR_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(cache) = guard.as_ref()
            && let Some(cached) = cache.get(&cache_key)
        {
            return filter_by_path(cached, file_path, project_root);
        }
    }

    let mut rules = Vec::new();

    collect_hierarchy_rules(dir, root, &mut rules);
    collect_glob_rules(project_root, client_id, &mut rules);

    {
        let mut guard = DIR_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cache = guard.get_or_insert_with(HashMap::new);
        cache.insert(cache_key, rules.clone());
    }

    filter_by_path(&rules, file_path, project_root)
}

/// Walk from `dir` up to `root`, collecting CLAUDE.md and AGENTS.md files.
fn collect_hierarchy_rules(dir: &Path, root: &Path, rules: &mut Vec<DiscoveredRule>) {
    let mut current = Some(dir);
    while let Some(d) = current {
        for name in &["CLAUDE.md", "AGENTS.md"] {
            let candidate = d.join(name);
            if candidate.is_file()
                && let Ok(content) = std::fs::read_to_string(&candidate)
                && !content.trim().is_empty()
            {
                let relative = candidate.strip_prefix(root).map_or_else(
                    |_| candidate.display().to_string(),
                    |p| p.display().to_string(),
                );
                rules.push(DiscoveredRule {
                    source: relative,
                    content,
                });
            }
        }
        if d == root {
            break;
        }
        current = d.parent();
    }
}

/// Collect glob-scoped rules from .claude/rules and .cursor/rules.
fn collect_glob_rules(project_root: &str, client_id: &str, rules: &mut Vec<DiscoveredRule>) {
    let root = Path::new(project_root);

    let rule_dirs: Vec<PathBuf> = if client_id.contains("cursor") {
        vec![root.join(".cursor/rules")]
    } else if client_id.contains("claude") {
        vec![root.join(".claude/rules")]
    } else {
        vec![root.join(".cursor/rules"), root.join(".claude/rules")]
    };

    for rule_dir in rule_dirs {
        if !rule_dir.is_dir() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&rule_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "md" | "mdc") {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                if content.trim().is_empty() {
                    continue;
                }
                let relative = path
                    .strip_prefix(root)
                    .map_or_else(|_| path.display().to_string(), |p| p.display().to_string());
                rules.push(DiscoveredRule {
                    source: relative,
                    content,
                });
            }
        }
    }
}

/// Filter rules to those applicable to the given file path.
/// Hierarchy rules (CLAUDE.md, AGENTS.md) always apply.
/// Glob-scoped rules (.cursor/rules, .claude/rules) apply only if their
/// path/globs pattern matches the file.
fn filter_by_path(
    rules: &[DiscoveredRule],
    file_path: &str,
    project_root: &str,
) -> Vec<DiscoveredRule> {
    let relative_file = Path::new(file_path)
        .strip_prefix(project_root)
        .map_or_else(|_| file_path.to_string(), |p| p.display().to_string());

    rules
        .iter()
        .filter(|r| {
            let src = &r.source;
            if src.ends_with("CLAUDE.md") || src.ends_with("AGENTS.md") {
                return true;
            }
            match extract_globs(&r.content) {
                Some(patterns) => patterns.iter().any(|p| glob_matches(p, &relative_file)),
                None => true,
            }
        })
        .cloned()
        .collect()
}

/// Extract glob patterns from rule file frontmatter.
/// Supports both Cursor `.mdc` format (`globs: pattern`) and Claude Code
/// `.claude/rules/*.md` format (`path: pattern`).
fn extract_globs(content: &str) -> Option<Vec<String>> {
    if !content.starts_with("---") {
        return None;
    }
    let end = content[3..].find("---")?;
    let frontmatter = &content[3..3 + end];

    let mut patterns = Vec::new();
    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("globs:")
            .or_else(|| trimmed.strip_prefix("path:"))
        {
            let pattern = rest.trim().trim_matches('"').trim_matches('\'');
            if !pattern.is_empty() {
                for p in pattern.split(',') {
                    let p = p.trim();
                    if !p.is_empty() {
                        patterns.push(p.to_string());
                    }
                }
            }
        }
    }

    if patterns.is_empty() {
        None
    } else {
        Some(patterns)
    }
}

/// Simple glob matching (supports `*` and `**`).
fn glob_matches(pattern: &str, path: &str) -> bool {
    if pattern == "*" || pattern == "**/*" || pattern == "**" {
        return true;
    }

    let pattern = pattern.replace('\\', "/");
    let path = path.replace('\\', "/");

    if let Some(suffix) = pattern.strip_prefix("**/") {
        if !suffix.contains('/') {
            let filename = path.rsplit('/').next().unwrap_or(&path);
            return glob_matches(suffix, filename);
        }
        let mut remaining = &path[..];
        loop {
            if glob_matches(suffix, remaining) {
                return true;
            }
            match remaining.find('/') {
                Some(pos) => remaining = &remaining[pos + 1..],
                None => return false,
            }
        }
    }

    if pattern.starts_with("*.") {
        let ext = &pattern[1..];
        return path.ends_with(ext);
    }

    if !pattern.contains('*') {
        return path == pattern || path.ends_with(&format!("/{pattern}"));
    }

    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 2 {
        let (prefix, suffix) = (parts[0], parts[1]);
        return (prefix.is_empty() || path.starts_with(prefix))
            && (suffix.is_empty() || path.ends_with(suffix));
    }

    path.contains(pattern.trim_matches('*'))
}

fn blake3_short(content: &str) -> String {
    let hash = blake3::hash(content.as_bytes());
    hash.to_hex()[..8].to_string()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches_extension() {
        assert!(glob_matches("*.rs", "src/main.rs"));
        assert!(glob_matches("*.rs", "main.rs"));
        assert!(!glob_matches("*.rs", "src/main.py"));
    }

    #[test]
    fn glob_matches_double_star() {
        assert!(glob_matches("**/test_*.rs", "src/tests/test_foo.rs"));
        assert!(glob_matches("**/test_*.rs", "test_foo.rs"));
        assert!(!glob_matches("**/test_*.rs", "src/foo.rs"));
    }

    #[test]
    fn glob_matches_exact() {
        assert!(glob_matches("src/main.rs", "src/main.rs"));
        assert!(!glob_matches("src/main.rs", "src/lib.rs"));
    }

    #[test]
    fn extract_globs_cursor_mdc() {
        let content = "---\nglobs: \"*.rs, *.toml\"\n---\nSome rule content";
        let patterns = extract_globs(content).unwrap();
        assert_eq!(patterns, vec!["*.rs", "*.toml"]);
    }

    #[test]
    fn extract_globs_claude_path() {
        let content = "---\npath: src/**/*.ts\n---\nRule for TS files";
        let patterns = extract_globs(content).unwrap();
        assert_eq!(patterns, vec!["src/**/*.ts"]);
    }

    #[test]
    fn extract_globs_no_frontmatter() {
        let content = "Just a plain markdown rule file";
        assert!(extract_globs(content).is_none());
    }

    #[test]
    fn format_suffix_empty_when_no_rules() {
        reset_injection_cache();
        let suffix = rules_suffix_for_read("/nonexistent/file.rs", "/nonexistent", "cursor");
        assert!(suffix.is_empty());
    }

    #[test]
    fn dedup_prevents_reinjection() {
        reset_injection_cache();
        let key = "test:abc12345";
        assert!(!already_injected(key));
        assert!(already_injected(key));
    }
}
