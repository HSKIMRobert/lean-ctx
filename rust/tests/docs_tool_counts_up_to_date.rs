use std::path::PathBuf;

#[test]
fn docs_tool_counts_match_manifest() {
    let expected_granular = lean_ctx::tool_defs::granular_tool_defs().len();
    let expected_unified = lean_ctx::tool_defs::unified_tool_defs().len();

    let rust_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = rust_dir.parent().unwrap_or(&rust_dir);

    let checks: Vec<(&str, Vec<String>)> = vec![
        (
            "README.md",
            vec![format!("· {} tools ·", expected_granular)],
        ),
        (
            "ARCHITECTURE.md",
            vec![format!("Context Server ({} tools)", expected_granular)],
        ),
        (
            "VISION.md",
            vec![format!("{} MCP tools", expected_granular)],
        ),
        (
            "LEANCTX_FEATURE_CATALOG.md",
            vec![
                format!("Granular MCP tools: **{}**", expected_granular),
                format!("Unified MCP tools: **{}**", expected_unified),
                format!("## Granular MCP Tools ({})", expected_granular),
            ],
        ),
        (
            "rust/README.md",
            vec![
                format!("{} MCP tools", expected_granular),
                format!("## {} MCP Tools", expected_granular),
            ],
        ),
        (
            "skills/lean-ctx/SKILL.md",
            vec![format!("— {} MCP tools", expected_granular)],
        ),
        (
            "rust/src/templates/SKILL.md",
            vec![format!("— {} MCP tools", expected_granular)],
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (rel, must_contain) in checks {
        let path = repo_root.join(rel);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        for needle in must_contain {
            if !content.contains(&needle) {
                failures.push(format!("{rel}: missing `{needle}`"));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "docs/tool-count drift detected:\n{}",
        failures.join("\n")
    );
}
