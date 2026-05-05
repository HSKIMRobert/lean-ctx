fn extract_arm_body<'a>(src: &'a str, tool: &str) -> Option<&'a str> {
    let needle = format!("\"{tool}\" => {{");
    let start = src.find(&needle)?;
    let brace_start = src[start..].find('{')? + start;
    let mut depth = 0u32;
    for (i, ch) in src[brace_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = brace_start + i + 1;
                    return Some(&src[brace_start..end]);
                }
            }
            _ => {}
        }
    }
    None
}

#[test]
fn server_fs_tools_use_resolve_path_chokepoint() {
    let sources = [
        include_str!("../src/server/dispatch/read_tools.rs"),
        include_str!("../src/server/dispatch/shell_tools.rs"),
        include_str!("../src/server/dispatch/session_tools.rs"),
        include_str!("../src/server/dispatch/utility_tools.rs"),
    ];
    let src = sources.join("\n");

    // Tools dispatched via the legacy match-cascade must call resolve_path
    // directly in their arm body.
    let legacy_tools = [
        "ctx_read",
        "ctx_multi_read",
        "ctx_search",
        "ctx_smart_read",
        "ctx_delta",
        "ctx_edit",
        "ctx_fill",
        "ctx_semantic_search",
        "ctx_prefetch",
        "ctx_cache",
        "ctx_graph",
        "ctx_handoff",
        "ctx_execute",
    ];
    for t in legacy_tools {
        let body = extract_arm_body(&src, t).unwrap_or_else(|| panic!("missing tool arm: {t}"));
        assert!(
            body.contains("resolve_path("),
            "{t} arm must call resolve_path() for path arguments"
        );
    }

    // Tools migrated to the ToolRegistry get resolve_path automatically
    // via dispatch_inner's pre-resolution loop. Verify the loop exists.
    let dispatch_mod = include_str!("../src/server/dispatch/mod.rs");
    assert!(
        dispatch_mod.contains("self.resolve_path(raw)"),
        "dispatch_inner must resolve paths for registry-dispatched tools"
    );

    // Verify registry-migrated tools use resolved_path from ToolContext
    let registry_tools = [
        (
            "ctx_tree",
            include_str!("../src/tools/registered/ctx_tree.rs"),
        ),
        (
            "ctx_benchmark",
            include_str!("../src/tools/registered/ctx_benchmark.rs"),
        ),
        (
            "ctx_analyze",
            include_str!("../src/tools/registered/ctx_analyze.rs"),
        ),
        (
            "ctx_outline",
            include_str!("../src/tools/registered/ctx_outline.rs"),
        ),
        (
            "ctx_review",
            include_str!("../src/tools/registered/ctx_review.rs"),
        ),
        (
            "ctx_impact",
            include_str!("../src/tools/registered/ctx_impact.rs"),
        ),
        (
            "ctx_architecture",
            include_str!("../src/tools/registered/ctx_architecture.rs"),
        ),
        (
            "ctx_pack",
            include_str!("../src/tools/registered/ctx_pack.rs"),
        ),
        (
            "ctx_index",
            include_str!("../src/tools/registered/ctx_index.rs"),
        ),
        (
            "ctx_artifacts",
            include_str!("../src/tools/registered/ctx_artifacts.rs"),
        ),
        (
            "ctx_compress_memory",
            include_str!("../src/tools/registered/ctx_compress_memory.rs"),
        ),
    ];
    for (name, src) in registry_tools {
        assert!(
            src.contains("resolved_path("),
            "{name}: registry tool must use ctx.resolved_path() for path access"
        );
    }
}
