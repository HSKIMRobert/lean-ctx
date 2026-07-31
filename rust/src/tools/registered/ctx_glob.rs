use rmcp::ErrorData;
use rmcp::model::Tool;
use serde_json::{Map, Value, json};

use crate::core::ocla::cache_types::{CacheKey, CacheKeyBuilder, DirectoryWalkKey};
use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput, get_bool, get_int, get_str};
use crate::tool_defs::tool_def;

pub struct CtxGlobTool;

impl McpTool for CtxGlobTool {
    fn name(&self) -> &'static str {
        "ctx_glob"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_glob",
            "Find files by glob pattern (respects .gitignore; multi-root via paths).\n\
             For file CONTENT search use ctx_search.",
            json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "e.g. **/*.ts" },
                    "path": { "type": "string" },
                    "paths": { "type": "array", "items": { "type": "string" } },
                    "max_results": { "type": "integer", "description": "default 200" },
                    "ignore_gitignore": { "type": "boolean", "description": "Requires admin role" }
                },
                "required": ["pattern"]
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let pattern = get_str(args, "pattern")
            .ok_or_else(|| ErrorData::invalid_params("pattern is required", None))?;
        let resolved = crate::server::multi_path::resolve_tool_paths(args, ctx)
            .map_err(|e| ErrorData::invalid_params(format!("ERROR: {e}"), None))?;
        let max = (get_int(args, "max_results").unwrap_or(200) as usize).min(500);
        let no_gitignore = get_bool(args, "ignore_gitignore").unwrap_or(false);

        if no_gitignore
            && let Err(e) = crate::core::io_boundary::ensure_ignore_gitignore_allowed("ctx_glob")
        {
            return Ok(ToolOutput::simple(e));
        }

        let respect = !no_gitignore;
        let allow_secret_paths = crate::core::roles::active_role().io.allow_secret_paths;

        if !resolved.is_multi {
            return handle_single(
                &pattern,
                &resolved.roots[0],
                respect,
                allow_secret_paths,
                max,
            );
        }

        let _mode_guard = crate::core::savings_footer::ModeGuard::new("glob");
        let per_root_max = (max / resolved.roots.len()).max(5);
        let mut combined = String::new();
        let mut total_original: usize = 0;
        let mut total_sent: usize = 0;

        for root in &resolved.roots {
            // The dispatch layer already runs `handle()` inside `block_in_place`
            // (server/dispatch/mod.rs); the per-root walk is synchronous, so we
            // call it directly and only guard against panics — nesting another
            // `block_in_place` here would needlessly consume blocking-pool
            // threads (the lesson from the ctx_multi_read crash, #271).
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                cached_or_walk(&pattern, root, respect, allow_secret_paths, per_root_max)
            }));

            let Ok((result, original)) = result else {
                combined.push_str(&format!("── {root} ──\nERROR: internal panic\n\n"));
                continue;
            };

            combined.push_str(&format!("── {root} ──\n{result}\n\n"));
            if !result.starts_with("ERROR:") {
                total_original += original;
                total_sent += crate::core::tokens::count_tokens(&result);
            }
        }

        let final_out =
            crate::core::protocol::append_savings(&combined, total_original, total_sent);
        let saved = total_original.saturating_sub(total_sent);

        Ok(ToolOutput {
            text: final_out,
            original_tokens: total_original,
            saved_tokens: saved,
            mode: None,
            path: None,
            changed: false,
            shell_outcome: None,
            content_blocks: None,
        })
    }
}

fn handle_single(
    pattern: &str,
    path: &str,
    respect_gitignore: bool,
    allow_secret_paths: bool,
    max_results: usize,
) -> Result<ToolOutput, ErrorData> {
    let Ok((result, original)) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cached_or_walk(
            pattern,
            path,
            respect_gitignore,
            allow_secret_paths,
            max_results,
        )
    })) else {
        return Err(ErrorData::internal_error(
            format!(
                "ctx_glob panicked while processing '{path}'. This is a bug — please report it."
            ),
            None,
        ));
    };

    if result.starts_with("ERROR:") {
        return Err(ErrorData::invalid_params(result, None));
    }

    let sent = crate::core::tokens::count_tokens(&result);
    let saved = original.saturating_sub(sent);
    let final_out = crate::core::protocol::append_savings(&result, original, sent);

    Ok(ToolOutput {
        text: final_out,
        original_tokens: original,
        saved_tokens: saved,
        mode: None,
        path: Some(path.to_string()),
        changed: false,
        shell_outcome: None,
        content_blocks: None,
    })
}

/// Builds the versioned directory-walk cache key for a glob request.
fn glob_cache_key(pattern: &str, path: &str, depth: usize) -> CacheKey {
    glob_cache_builder(pattern, path, depth, true, false).cache_key()
}

fn glob_cache_builder(
    _pattern: &str,
    path: &str,
    depth: usize,
    respect_gitignore: bool,
    _allow_secret_paths: bool,
) -> DirectoryWalkKey {
    let canonical = crate::core::pathutil::safe_canonicalize_or_self(std::path::Path::new(path));
    let dir_mtime_ns = directory_mtime_ns(&canonical).unwrap_or_default();
    DirectoryWalkKey {
        path: canonical.to_string_lossy().into_owned(),
        depth,
        gitignore: respect_gitignore,
        dir_mtime_ns,
    }
}

fn directory_mtime_ns(path: &std::path::Path) -> Option<u128> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos())
}

fn cached_or_walk(
    pattern: &str,
    path: &str,
    respect_gitignore: bool,
    allow_secret_paths: bool,
    max_results: usize,
) -> (String, usize) {
    // Glob has no explicit depth limit; preserve that in the key instead of
    // accidentally sharing a limited tree result.
    let selector = format!("{pattern}\\x1fmax:{max_results}");
    let builder = glob_cache_builder(
        &selector,
        path,
        usize::MAX,
        respect_gitignore,
        allow_secret_paths,
    );
    let key = if respect_gitignore && allow_secret_paths {
        glob_cache_key(&selector, path, usize::MAX)
    } else {
        builder.cache_key()
    };
    if let Some(entry) =
        crate::core::ocla::cache_delivery::check(&key, &builder.validator(), "ctx_glob")
    {
        let stub = crate::core::ocla::cache_delivery::stub(&entry, "directory walk");
        return (stub, entry.token_count as usize);
    }

    let (result, original) = crate::tools::ctx_glob::handle(
        pattern,
        path,
        respect_gitignore,
        allow_secret_paths,
        max_results,
    );
    if !result.starts_with("ERROR:") {
        crate::core::ocla::cache_delivery::record(
            key,
            crate::core::ocla::cache_types::DeliveryKind::DirectoryWalk,
            builder.validator(),
            Some(builder.path),
            &result,
            "ctx_glob",
        );
    }
    (result, original)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_adapter_records_then_serves_a_cross_agent_reference() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("cached.rs"), "fn cached() {}\n").unwrap();
        let path = directory.path().to_string_lossy();

        let first = handle_single("*.rs", &path, true, true, 20).unwrap();
        assert!(first.text.contains("cached.rs"));
        let second = handle_single("*.rs", &path, true, true, 20).unwrap();
        assert!(
            second.text.contains("[cross-agent cache"),
            "{}",
            second.text
        );
    }
}
