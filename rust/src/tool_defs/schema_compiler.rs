//! Token-efficient compilation of MCP tool definitions.

use std::collections::HashSet;

use rmcp::model::Tool;
use serde_json::{Map, Value};

/// A token-efficient representation of a tool definition.
#[derive(Debug, Clone)]
pub struct CompactToolDef {
    /// Tool name exposed through MCP.
    pub name: String,
    /// Compact parameter signature.
    pub signature: String,
    /// First sentence of the tool description.
    pub one_liner: String,
    /// Original input schema retained for on-demand retrieval.
    pub full_schema: Option<Map<String, Value>>,
}

/// Estimated token costs before and after schema compilation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompilationSavings {
    /// Estimated tokens used by full tool definitions.
    pub full_schema_tokens: usize,
    /// Estimated tokens used by the compiled summary pool.
    pub compiled_tokens: usize,
    /// Percentage reduction from full definitions to compiled summaries.
    pub savings_percent: f64,
}

/// Extract a compact function-style signature from a tool input schema.
#[must_use]
pub fn extract_signature(tool: &Tool) -> String {
    let required: HashSet<&str> = tool
        .input_schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let arguments = tool
        .input_schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| {
            properties
                .iter()
                .map(|(name, schema)| {
                    format_argument(name, schema, required.contains(name.as_str()))
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    format!("{}({arguments})", tool.name)
}

#[must_use]
fn format_argument(name: &str, schema: &Value, is_required: bool) -> String {
    let mut argument = name.to_string();
    if !is_required {
        argument.push('?');
    }

    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        let mut formatted = values
            .iter()
            .take(5)
            .map(format_schema_value)
            .collect::<Vec<_>>();
        if values.len() > 5 {
            formatted.push("...".to_string());
        }
        if !formatted.is_empty() {
            argument.push('=');
            argument.push_str(&formatted.join("|"));
        }
    } else if let Some(default) = schema.get("default") {
        argument.push('=');
        argument.push_str(&format_schema_value(default));
    }

    argument
}

#[must_use]
fn format_schema_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

/// Extract the first sentence of a description, capped at 80 characters.
#[must_use]
pub fn extract_one_liner(description: &str) -> String {
    let trimmed = description.trim_start();
    let end = trimmed
        .char_indices()
        .find_map(|(index, character)| matches!(character, '.' | '\n').then_some(index))
        .unwrap_or(trimmed.len());
    let sentence = trimmed[..end].trim_end();
    if sentence.chars().count() <= 80 {
        return sentence.to_string();
    }

    let mut truncated = sentence.chars().take(79).collect::<String>();
    truncated.push('…');
    truncated
}

/// Compile one MCP tool definition into its compact representation.
#[must_use]
pub fn compile_tool(tool: &Tool) -> CompactToolDef {
    CompactToolDef {
        name: tool.name.to_string(),
        signature: extract_signature(tool),
        one_liner: extract_one_liner(tool.description.as_deref().unwrap_or_default()),
        full_schema: Some((*tool.input_schema).clone()),
    }
}

/// Compile all tools and return them in deterministic name order.
#[must_use]
pub fn compile_all(tools: &[Tool]) -> Vec<CompactToolDef> {
    let mut compiled = tools.iter().map(compile_tool).collect::<Vec<_>>();
    compiled.sort_by(|left, right| left.name.cmp(&right.name));
    compiled
}

/// Format one compact tool definition for the summary pool.
#[must_use]
pub fn format_summary_line(compact: &CompactToolDef) -> String {
    format!("  {}: {}", compact.signature, compact.one_liner)
}

/// Format the complete deterministic tool summary pool.
#[must_use]
pub fn format_summary_pool(compacts: &[CompactToolDef]) -> String {
    let mut sorted = compacts.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.name.cmp(&right.name));
    let mut output = String::from("Available lean-ctx tools:");
    for compact in sorted {
        output.push('\n');
        output.push_str(&format_summary_line(compact));
    }
    output
}

/// Rough token estimate (1 token ≈ 4 chars for English).
#[must_use]
fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

/// Compare the estimated token cost of full schemas and compiled summaries.
#[must_use]
pub fn estimate_compilation_savings(tools: &[Tool]) -> CompilationSavings {
    let full_schema_tokens: usize = tools
        .iter()
        .map(|tool| {
            let schema = serde_json::to_string(tool.input_schema.as_ref()).unwrap_or_default();
            estimate_tokens(&schema)
                + estimate_tokens(tool.description.as_deref().unwrap_or_default())
        })
        .sum();
    let compiled_tokens = estimate_tokens(&format_summary_pool(&compile_all(tools)));
    let savings_percent = if full_schema_tokens == 0 {
        0.0
    } else {
        100.0 * (full_schema_tokens.saturating_sub(compiled_tokens) as f64)
            / full_schema_tokens as f64
    };

    CompilationSavings {
        full_schema_tokens,
        compiled_tokens,
        savings_percent,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        compile_all, compile_tool, estimate_compilation_savings, extract_one_liner,
        extract_signature, format_summary_pool,
    };
    use crate::tool_defs::tool_def;

    fn sample_tool(schema: serde_json::Value) -> rmcp::model::Tool {
        tool_def(
            "ctx_read",
            "Read file with compression. More detail.",
            schema,
        )
    }

    #[test]
    fn test_extract_signature_required_only() {
        let tool = sample_tool(json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        }));
        assert_eq!(extract_signature(&tool), "ctx_read(path)");
    }

    #[test]
    fn test_extract_signature_with_optionals() {
        let tool = sample_tool(json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "mode": {"type": "string", "default": "auto"}
            },
            "required": ["path"]
        }));
        assert_eq!(extract_signature(&tool), "ctx_read(mode?=auto, path)");
    }

    #[test]
    fn test_extract_signature_with_enums() {
        let tool = sample_tool(json!({
            "type": "object",
            "properties": {
                "mode": {"type": "string", "enum": ["auto", "full", "outline", "map", "raw", "diff"]}
            }
        }));
        assert_eq!(
            extract_signature(&tool),
            "ctx_read(mode?=auto|full|outline|map|raw|...)"
        );
    }

    #[test]
    fn test_extract_signature_boolean_implied() {
        let tool = sample_tool(json!({
            "type": "object",
            "properties": {"fresh": {"type": "boolean"}}
        }));
        assert_eq!(extract_signature(&tool), "ctx_read(fresh?)");
    }

    #[test]
    fn test_extract_one_liner_first_sentence() {
        assert_eq!(
            extract_one_liner("  First sentence. Second sentence."),
            "First sentence"
        );
    }

    #[test]
    fn test_extract_one_liner_truncation() {
        let compact = extract_one_liner(&"a".repeat(81));
        assert_eq!(compact.chars().count(), 80);
        assert!(compact.ends_with('…'));
    }

    #[test]
    fn test_compile_tool_round_trip() {
        let tool = sample_tool(json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        }));
        let compact = compile_tool(&tool);
        assert_eq!(compact.name, "ctx_read");
        assert_eq!(compact.signature, "ctx_read(path)");
        assert_eq!(compact.one_liner, "Read file with compression");
        assert_eq!(
            compact.full_schema.as_ref(),
            Some(tool.input_schema.as_ref())
        );
    }

    #[test]
    fn test_format_summary_pool() {
        let read = sample_tool(json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        }));
        let shell = tool_def(
            "ctx_shell",
            "Execute shell command with compression.",
            json!({
                "type": "object",
                "properties": {"command": {"type": "string"}},
                "required": ["command"]
            }),
        );
        assert_eq!(
            format_summary_pool(&compile_all(&[shell, read])),
            "Available lean-ctx tools:\n  ctx_read(path): Read file with compression\n  ctx_shell(command): Execute shell command with compression"
        );
    }

    #[test]
    fn test_savings_positive() {
        let tool = sample_tool(json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Absolute path to the file that should be read"},
                "mode": {"type": "string", "enum": ["auto", "full", "outline", "map", "raw", "diff"]},
                "fresh": {"type": "boolean", "description": "Bypass cached content when true"}
            },
            "required": ["path"]
        }));
        let savings = estimate_compilation_savings(&[tool]);
        assert!(savings.compiled_tokens < savings.full_schema_tokens);
        assert!(savings.savings_percent > 0.0);
    }
}
