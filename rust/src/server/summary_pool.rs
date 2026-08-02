//! Compact catalog of tools omitted from the lazy MCP surface.

use crate::server::dynamic_tools::{ToolCategory, categorize_tool, is_deprecated_alias};

const MAX_CATALOG_TOOLS: usize = 6;
const MAX_LINE_CHARS: usize = 80;

/// Generate a compact tool catalog for MCP instructions injection.
/// Uses the registry as single source of truth.
#[must_use]
pub fn generate_tool_catalog() -> String {
    generate_hidden_tool_catalog(&[])
}

/// Generate catalog filtered to only list tools NOT in the visible set.
/// This avoids duplication: tools the LLM already sees via tools/list
/// are excluded from the instructions catalog.
#[must_use]
pub fn generate_hidden_tool_catalog(visible_tools: &[&str]) -> String {
    let discovered = crate::tool_defs::discover_tools("");
    let mut tools: Vec<(&str, &str)> = discovered
        .lines()
        .filter_map(parse_discovery_line)
        .filter(|(name, _)| !visible_tools.contains(name))
        .filter(|(name, _)| categorize_tool(name) != ToolCategory::Internal)
        .filter(|(name, _)| !is_deprecated_alias(name))
        .collect();
    tools.sort_unstable_by_key(|(name, _)| *name);

    if tools.is_empty() {
        return String::new();
    }

    let shown = tools.len().min(MAX_CATALOG_TOOLS);
    let remaining = tools.len() - shown;
    let mut catalog = String::from("Additional tools (invoke via ctx_call):\n");
    for (name, description) in tools.into_iter().take(shown) {
        catalog.push_str(&catalog_line(name, description));
        catalog.push('\n');
    }
    catalog.push_str(&format!(
        "  ... (+{remaining} more — use ctx_discover_tools(\"query\") to search)"
    ));
    catalog
}

fn parse_discovery_line(line: &str) -> Option<(&str, &str)> {
    let entry = line.strip_prefix("  ctx_")?;
    let (name_suffix, description) = entry.split_once(" — ")?;
    Some((
        line.get(2..2 + "ctx_".len() + name_suffix.len())?,
        first_sentence(description),
    ))
}

fn first_sentence(description: &str) -> &str {
    description
        .find(['.', '!', '?'])
        .map_or(description, |end| &description[..=end])
}

fn catalog_line(name: &str, description: &str) -> String {
    let prefix = format!("  {name}({}): ", key_args(name));
    let available = MAX_LINE_CHARS.saturating_sub(prefix.chars().count());
    let shortened: String = description.chars().take(available).collect();
    format!("{prefix}{shortened}")
}

fn key_args(name: &str) -> &'static str {
    match name {
        "ctx_agent" => "action, id?",
        "ctx_architecture" => "action?, path?",
        "ctx_artifacts" => "action?, query?",
        "ctx_execute" => "language, code",
        "ctx_graph" => "action, path?",
        "ctx_handoff" => "action, agent?",
        "ctx_impact" => "symbol, kind?",
        "ctx_knowledge" => "action, query?",
        "ctx_plan" => "task",
        "ctx_provider" => "action, provider?",
        "ctx_refactor" => "path, symbol?",
        "ctx_review" => "path?",
        "ctx_share" => "action, target?",
        "ctx_task" => "action, task?",
        "ctx_verify" => "path?",
        "ctx_workflow" => "action, name?",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::{generate_hidden_tool_catalog, generate_tool_catalog};

    #[test]
    fn test_generate_tool_catalog_not_empty() {
        assert!(!generate_tool_catalog().is_empty());
    }

    #[test]
    fn test_generate_hidden_excludes_visible() {
        let catalog = generate_hidden_tool_catalog(&["ctx_architecture", "ctx_execute"]);
        assert!(!catalog.contains("ctx_architecture("));
        assert!(!catalog.contains("ctx_execute("));
    }

    #[test]
    fn test_generate_hidden_excludes_internal() {
        let catalog = generate_tool_catalog();
        for name in ["ctx_metrics", "ctx_cost", "ctx_feedback"] {
            assert!(!catalog.contains(name), "internal tool leaked: {name}");
        }
    }

    #[test]
    fn test_generate_hidden_excludes_deprecated() {
        let catalog = generate_tool_catalog();
        for name in ["ctx_multi_read", "ctx_smart_read", "ctx_semantic_search"] {
            assert!(!catalog.contains(name), "deprecated tool leaked: {name}");
        }
    }

    #[test]
    fn test_catalog_format() {
        let catalog = generate_tool_catalog();
        let lines: Vec<&str> = catalog.lines().collect();
        assert_eq!(lines[0], "Additional tools (invoke via ctx_call):");
        assert!(
            lines
                .last()
                .is_some_and(|line| line.contains(" more — use "))
        );
        for line in &lines[1..lines.len() - 1] {
            assert!(line.starts_with("  ctx_"));
            assert!(line.contains("): "));
            assert!(line.chars().count() <= 80, "line too long: {line}");
        }
    }
}
