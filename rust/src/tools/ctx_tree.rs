use std::path::Path;

use ignore::WalkBuilder;

use crate::core::protocol;
use crate::core::tokens::count_tokens;

/// Generates a compact directory tree listing with file counts.
/// When `respect_gitignore` is true, entries matching .gitignore patterns are excluded.
pub fn handle(
    path: &str,
    depth: usize,
    show_hidden: bool,
    respect_gitignore: bool,
) -> (String, usize) {
    let root = Path::new(path);
    if root.is_file() {
        let parent = root
            .parent()
            .map_or(path.to_string(), |p| p.display().to_string());
        return (
            format!(
                "ERROR: '{path}' is a file, not a directory. Use path=\"{parent}\" for the containing directory."
            ),
            0,
        );
    }
    if !root.is_dir() {
        return (
            format!("ERROR: {path} does not exist or is not a directory"),
            0,
        );
    }

    let raw_output = generate_raw_tree(root, depth, show_hidden, respect_gitignore);
    let compact_output = generate_compact_tree(root, depth, show_hidden, respect_gitignore);

    if compact_output.trim().is_empty() {
        return (format!("{path}/ (empty directory, depth={depth})"), 0);
    }

    let raw_tokens = count_tokens(&raw_output);
    let compact_tokens = count_tokens(&compact_output);
    let savings = protocol::format_savings(raw_tokens, compact_tokens);

    (format!("{compact_output}\n{savings}"), raw_tokens)
}

fn generate_compact_tree(
    root: &Path,
    max_depth: usize,
    show_hidden: bool,
    respect_gitignore: bool,
) -> String {
    let mut lines = Vec::new();

    struct Entry {
        depth: usize,
        name: String,
        is_dir: bool,
        path: std::path::PathBuf,
    }
    let mut entries: Vec<Entry> = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(!show_hidden)
        .git_ignore(respect_gitignore)
        .git_global(respect_gitignore)
        .git_exclude(respect_gitignore)
        .max_depth(Some(max_depth))
        .sort_by_file_name(std::cmp::Ord::cmp)
        .build();

    for entry in walker.filter_map(std::result::Result::ok) {
        if entry.depth() == 0 {
            continue;
        }
        entries.push(Entry {
            depth: entry.depth(),
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir: entry.file_type().is_some_and(|ft| ft.is_dir()),
            path: entry.path().to_path_buf(),
        });
    }

    let mut dir_file_counts: std::collections::HashMap<&std::path::Path, usize> =
        std::collections::HashMap::new();
    for e in &entries {
        if !e.is_dir {
            if let Some(parent) = e.path.parent() {
                *dir_file_counts.entry(parent).or_default() += 1;
            }
        }
    }

    for e in &entries {
        let indent = "  ".repeat(e.depth.saturating_sub(1));
        if e.is_dir {
            let count = dir_file_counts.get(e.path.as_path()).copied().unwrap_or(0);
            lines.push(format!("{indent}{}/ ({count})", e.name));
        } else {
            lines.push(format!("{indent}{}", e.name));
        }
    }

    lines.join("\n")
}

fn generate_raw_tree(
    root: &Path,
    depth: usize,
    show_hidden: bool,
    respect_gitignore: bool,
) -> String {
    let mut lines = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(!show_hidden)
        .git_ignore(respect_gitignore)
        .git_global(respect_gitignore)
        .git_exclude(respect_gitignore)
        .max_depth(Some(depth))
        .sort_by_file_name(std::cmp::Ord::cmp)
        .build();

    for entry in walker.filter_map(std::result::Result::ok) {
        if entry.depth() == 0 {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_string_lossy();
        lines.push(rel.to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_savings_are_reasonable() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let (output, original) = handle(dir, 3, false, true);
        let compact_tokens = count_tokens(&output);

        eprintln!("=== ctx_tree savings test ===");
        eprintln!("  original (raw) tokens: {original}");
        eprintln!("  compact tokens:        {compact_tokens}");
        eprintln!(
            "  savings:               {}",
            original.saturating_sub(compact_tokens)
        );

        assert!(
            original < 5000,
            "raw tree at depth 3 should be < 5000 tokens, got {original}"
        );
        assert!(original > 0, "raw tree should have some tokens");
        if original > compact_tokens {
            let ratio = (original - compact_tokens) as f64 / original as f64;
            eprintln!("  savings ratio:         {:.1}%", ratio * 100.0);
            assert!(
                ratio < 0.90,
                "savings ratio should be < 90% for same-depth comparison, got {:.1}%",
                ratio * 100.0
            );
        }
    }
}
