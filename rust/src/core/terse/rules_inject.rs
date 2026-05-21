//! Injects compression prompts into agent rules files for all integration modes.
//!
//! Called from:
//! - `lean-ctx compression <level>` (CLI command)
//! - `lean-ctx setup` (initial setup)
//! - MCP server startup (ensures consistency after manual config edits)

use crate::core::config::CompressionLevel;

const COMPRESSION_BLOCK_START: &str = "<!-- lean-ctx-compression -->";
const COMPRESSION_BLOCK_END: &str = "<!-- /lean-ctx-compression -->";

/// Updates all detected agent rules files with the compression prompt for `level`.
/// Idempotent — safe to call repeatedly. Returns the number of files updated.
pub fn inject(level: &CompressionLevel) -> usize {
    let prompt = super::agent_prompts::build_prompt_block(level);
    let prompt_ascii = super::agent_prompts::build_prompt_block_for_client(level, "cursor");
    let block = |p: &str| {
        if p.is_empty() {
            String::new()
        } else {
            format!("{COMPRESSION_BLOCK_START}\n{p}\n{COMPRESSION_BLOCK_END}")
        }
    };

    let home = crate::core::home::resolve_home_dir().unwrap_or_default();
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut updated = 0;

    let cursor_paths: Vec<std::path::PathBuf> = vec![
        home.join(".cursor/rules/lean-ctx.mdc"),
        cwd.join(".cursorrules"),
    ];
    let other_paths: Vec<std::path::PathBuf> = vec![
        cwd.join("AGENTS.md"),
        cwd.join(".claude/rules/lean-ctx.md"),
        cwd.join(".kiro/steering/lean-ctx.md"),
        home.join(".config/crush/rules/lean-ctx.md"),
        home.join(".qoder/rules/lean-ctx.md"),
    ];

    for path in cursor_paths {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let new_content = upsert_block(&content, &block(&prompt_ascii));
                if new_content != content {
                    let _ = std::fs::write(&path, &new_content);
                    updated += 1;
                }
            }
        }
    }

    for path in other_paths {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let new_content = upsert_block(&content, &block(&prompt));
                if new_content != content {
                    let _ = std::fs::write(&path, &new_content);
                    updated += 1;
                }
            }
        }
    }

    updated
}

fn upsert_block(content: &str, block: &str) -> String {
    if content.contains(COMPRESSION_BLOCK_START) {
        crate::marked_block::replace_marked_block(
            content,
            COMPRESSION_BLOCK_START,
            COMPRESSION_BLOCK_END,
            block,
        )
    } else if block.is_empty() {
        content.to_string()
    } else {
        let mut out = content.trim_end().to_string();
        out.push_str("\n\n");
        out.push_str(block);
        out.push('\n');
        out
    }
}
