//! Compact MCP input schemas without changing their validation semantics.

use rmcp::model::Tool;
use serde_json::{Map, Value};
use std::sync::Arc;

/// Estimated token costs before and after applying schema diet.
#[derive(Debug, Clone, Copy)]
pub struct DietSavings {
    /// Estimated tokens used by the original input schemas.
    pub before_tokens: usize,
    /// Estimated tokens used by the compacted input schemas.
    pub after_tokens: usize,
    /// Percentage reduction in estimated schema tokens.
    pub savings_percent: f64,
}

/// Apply schema diet to a list of tools: strip property descriptions,
/// remove redundant type annotations, and compact enum representations.
/// Does NOT modify tool names or top-level descriptions (those are handled
/// by mcp_compress and schema_hook).
#[must_use]
pub fn apply_schema_diet(tools: Vec<Tool>) -> Vec<Tool> {
    tools.into_iter().map(diet_tool).collect()
}

/// Estimate token savings from applying schema diet to a set of tools.
#[must_use]
pub fn estimate_diet_savings(tools: &[Tool]) -> DietSavings {
    let before_tokens: usize = tools.iter().map(schema_tokens).sum();
    let dieted = apply_schema_diet(tools.to_vec());
    let after_tokens: usize = dieted.iter().map(schema_tokens).sum();
    let savings_percent = if before_tokens == 0 {
        0.0
    } else {
        100.0 * before_tokens.saturating_sub(after_tokens) as f64 / before_tokens as f64
    };

    DietSavings {
        before_tokens,
        after_tokens,
        savings_percent,
    }
}

#[must_use]
fn diet_tool(mut tool: Tool) -> Tool {
    let mut schema = (*tool.input_schema).clone();
    strip_property_descriptions(&mut schema);
    tool.input_schema = Arc::new(schema);
    tool
}

fn strip_property_descriptions(schema: &mut Map<String, Value>) {
    let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) else {
        return;
    };

    for property in properties.values_mut() {
        strip_schema_node(property, true);
    }
}

fn strip_schema_node(schema: &mut Value, remove_description: bool) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };

    if remove_description {
        object.remove("description");
    }

    if let Some(properties) = object.get_mut("properties").and_then(Value::as_object_mut) {
        for property in properties.values_mut() {
            strip_schema_node(property, true);
        }
    }

    if let Some(items) = object.get_mut("items") {
        strip_schema_node(items, true);
    }
}

#[must_use]
fn schema_tokens(tool: &Tool) -> usize {
    serde_json::to_string(tool.input_schema.as_ref())
        .unwrap_or_default()
        .len()
        .div_ceil(4)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{apply_schema_diet, estimate_diet_savings};
    use crate::tool_defs::tool_def;

    fn sample_tool(schema: Value) -> rmcp::model::Tool {
        tool_def("ctx_sample", "Sample tool.", schema)
    }

    fn diet(schema: Value) -> rmcp::model::Tool {
        apply_schema_diet(vec![sample_tool(schema)]).remove(0)
    }

    #[test]
    fn test_strip_removes_descriptions() {
        let tool = diet(json!({
            "type": "object",
            "properties": {"path": {"type": "string", "description": "A path"}}
        }));
        assert!(
            tool.input_schema["properties"]["path"]
                .get("description")
                .is_none()
        );
    }

    #[test]
    fn test_strip_preserves_types() {
        let tool = diet(json!({
            "type": "object",
            "properties": {"fresh": {"type": "boolean", "description": "Refresh"}}
        }));
        assert_eq!(tool.input_schema["type"], "object");
        assert_eq!(tool.input_schema["properties"]["fresh"]["type"], "boolean");
    }

    #[test]
    fn test_strip_preserves_enums() {
        let tool = diet(json!({
            "type": "object",
            "properties": {"mode": {"type": "string", "enum": ["auto", "raw"], "description": "Mode"}}
        }));
        assert_eq!(
            tool.input_schema["properties"]["mode"]["enum"],
            json!(["auto", "raw"])
        );
    }

    #[test]
    fn test_strip_preserves_defaults() {
        let tool = diet(json!({
            "type": "object",
            "properties": {"fresh": {"type": "boolean", "default": false, "description": "Refresh"}}
        }));
        assert_eq!(tool.input_schema["properties"]["fresh"]["default"], false);
    }

    #[test]
    fn test_strip_preserves_required() {
        let tool = diet(json!({
            "type": "object",
            "properties": {"path": {"type": "string", "description": "Path"}},
            "required": ["path"]
        }));
        assert_eq!(tool.input_schema["required"], json!(["path"]));
    }

    #[test]
    fn test_strip_handles_nested() {
        let tool = diet(json!({
            "type": "object",
            "properties": {
                "config": {"type": "object", "description": "Config", "properties": {
                    "name": {"type": "string", "description": "Name"}
                }},
                "entries": {"type": "array", "items": {
                    "type": "object", "description": "Entry", "properties": {
                        "value": {"type": "integer", "description": "Value"}
                    }
                }}
            }
        }));
        assert!(
            tool.input_schema["properties"]["config"]["properties"]["name"]
                .get("description")
                .is_none()
        );
        assert!(
            tool.input_schema["properties"]["entries"]["items"]
                .get("description")
                .is_none()
        );
        assert!(
            tool.input_schema["properties"]["entries"]["items"]["properties"]["value"]
                .get("description")
                .is_none()
        );
    }

    #[test]
    fn test_diet_savings_positive() {
        let tools = crate::server::registry::build_registry().tool_defs();
        let savings = estimate_diet_savings(&tools);
        assert!(savings.after_tokens < savings.before_tokens);
        assert!(savings.savings_percent > 0.0);
    }

    #[test]
    fn test_diet_preserves_tool_name() {
        let tool = sample_tool(json!({"type": "object", "properties": {}}));
        let name = tool.name.clone();
        let dieted = apply_schema_diet(vec![tool]);
        assert_eq!(dieted[0].name, name);
    }

    #[test]
    fn test_strip_preserves_combinators() {
        let combinator = json!([{"type": "string", "description": "Variant"}]);
        let tool = diet(json!({
            "type": "object",
            "properties": {"value": {"description": "Value", "anyOf": combinator.clone()}}
        }));
        assert_eq!(
            tool.input_schema["properties"]["value"]["anyOf"],
            combinator
        );
    }
}
