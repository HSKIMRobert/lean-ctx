use std::path::Path;
use walkdir::WalkDir;

use crate::core::protocol;
use crate::core::tokens::count_tokens;

pub fn handle(path: &str, depth: usize, show_hidden: bool) -> String {
    let root = Path::new(path);
    if !root.is_dir() {
        return format!("ERROR: {path} is not a directory");
    }

    let raw_output = generate_raw_tree(root);
    let compact_output = generate_compact_tree(root, depth, show_hidden);

    let raw_tokens = count_tokens(&raw_output);
    let compact_tokens = count_tokens(&compact_output);
    let savings = protocol::format_savings(raw_tokens, compact_tokens);

    format!("{compact_output}\n{savings}")
}

fn generate_compact_tree(root: &Path, max_depth: usize, show_hidden: bool) -> String {
    let mut lines = Vec::new();
    let mut entries: Vec<(usize, String, bool, usize)> = Vec::new();

    for entry in WalkDir::new(root)
        .min_depth(1)
        .max_depth(max_depth)
        .sort_by_file_name()
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        if is_ignored(&name) {
            continue;
        }

        let depth = entry.depth();
        let is_dir = entry.file_type().is_dir();

        let file_count = if is_dir {
            count_files_in_dir(entry.path(), show_hidden)
        } else {
            0
        };

        entries.push((depth, name, is_dir, file_count));
    }

    for (depth, name, is_dir, file_count) in &entries {
        let indent = "  ".repeat(depth.saturating_sub(1));
        if *is_dir {
            lines.push(format!("{indent}{name}/ ({file_count})"));
        } else {
            lines.push(format!("{indent}{name}"));
        }
    }

    lines.join("\n")
}

fn generate_raw_tree(root: &Path) -> String {
    let mut lines = Vec::new();

    for entry in WalkDir::new(root).min_depth(1).sort_by_file_name() {
        if let Ok(e) = entry {
            let name = e.file_name().to_string_lossy().to_string();
            if is_ignored(&name) {
                continue;
            }
            lines.push(
                e.path()
                    .strip_prefix(root)
                    .unwrap_or(e.path())
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }

    lines.join("\n")
}

fn count_files_in_dir(dir: &Path, show_hidden: bool) -> usize {
    WalkDir::new(dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            !e.file_type().is_dir() && (show_hidden || !name.starts_with('.')) && !is_ignored(&name)
        })
        .count()
}

fn is_ignored(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | ".git"
            | "target"
            | "dist"
            | "build"
            | ".next"
            | ".nuxt"
            | "__pycache__"
            | ".cache"
            | "coverage"
            | ".DS_Store"
            | "Thumbs.db"
    )
}
