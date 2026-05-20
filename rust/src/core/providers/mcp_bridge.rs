//! MCP Bridge provider — connects external MCP servers as data sources.
//!
//! Allows lean-ctx to query resources from other MCP servers (e.g., a
//! custom internal knowledge base MCP server) and integrate them into
//! the context pipeline.
//!
//! Configuration via lean-ctx config:
//!   [providers.mcp_bridges]
//!   my-kb = { url = "http://localhost:8080", description = "Internal KB" }

use crate::core::providers::{ContextProvider, ProviderItem, ProviderParams, ProviderResult};

pub struct McpBridgeProvider {
    server_url: String,
    server_name: String,
}

impl McpBridgeProvider {
    pub fn new(server_name: &str, server_url: &str) -> Self {
        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            server_name: server_name.to_string(),
        }
    }
}

impl ContextProvider for McpBridgeProvider {
    fn id(&self) -> &'static str {
        "mcp_bridge"
    }

    fn display_name(&self) -> &'static str {
        "MCP Bridge"
    }

    fn supported_actions(&self) -> &[&str] {
        &["resources", "tools"]
    }

    fn execute(&self, action: &str, params: &ProviderParams) -> Result<ProviderResult, String> {
        match action {
            "resources" => list_resources(&self.server_url, &self.server_name, params),
            "tools" => list_tools(&self.server_url, &self.server_name, params),
            _ => Err(format!("Unsupported action: {action}")),
        }
    }

    fn cache_ttl_secs(&self) -> u64 {
        60
    }

    fn requires_auth(&self) -> bool {
        false
    }

    fn is_available(&self) -> bool {
        !self.server_url.is_empty()
    }
}

fn list_resources(
    server_url: &str,
    server_name: &str,
    params: &ProviderParams,
) -> Result<ProviderResult, String> {
    let limit = params.limit.unwrap_or(20);
    let url = format!("{server_url}/resources/list");

    let response = ureq::get(&url)
        .header("Accept", "application/json")
        .call()
        .map_err(|e| format!("MCP bridge error ({server_name}): {e}"))?;

    let text = response
        .into_body()
        .read_to_string()
        .map_err(|e| format!("MCP bridge read error: {e}"))?;
    let body: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("MCP bridge JSON error: {e}"))?;

    let resources = body["resources"].as_array().cloned().unwrap_or_default();

    let items: Vec<ProviderItem> = resources
        .iter()
        .take(limit)
        .map(|r| ProviderItem {
            id: r["uri"].as_str().unwrap_or_default().to_string(),
            title: r["name"].as_str().unwrap_or_default().to_string(),
            state: Some("available".into()),
            author: None,
            created_at: None,
            updated_at: None,
            url: Some(format!("{server_url}/resources/read")),
            labels: vec![server_name.to_string()],
            body: r["description"].as_str().map(String::from),
        })
        .collect();

    Ok(ProviderResult {
        provider: format!("mcp_bridge:{server_name}"),
        resource_type: "resources".into(),
        items,
        total_count: Some(resources.len()),
        truncated: resources.len() > limit,
    })
}

fn list_tools(
    server_url: &str,
    server_name: &str,
    params: &ProviderParams,
) -> Result<ProviderResult, String> {
    let limit = params.limit.unwrap_or(20);
    let url = format!("{server_url}/tools/list");

    let response = ureq::get(&url)
        .header("Accept", "application/json")
        .call()
        .map_err(|e| format!("MCP bridge error ({server_name}): {e}"))?;

    let text = response
        .into_body()
        .read_to_string()
        .map_err(|e| format!("MCP bridge read error: {e}"))?;
    let body: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("MCP bridge JSON error: {e}"))?;

    let tools = body["tools"].as_array().cloned().unwrap_or_default();

    let items: Vec<ProviderItem> = tools
        .iter()
        .take(limit)
        .map(|t| ProviderItem {
            id: t["name"].as_str().unwrap_or_default().to_string(),
            title: t["name"].as_str().unwrap_or_default().to_string(),
            state: Some("available".into()),
            author: None,
            created_at: None,
            updated_at: None,
            url: Some(format!("{server_url}/tools/call")),
            labels: vec![server_name.to_string()],
            body: t["description"].as_str().map(String::from),
        })
        .collect();

    Ok(ProviderResult {
        provider: format!("mcp_bridge:{server_name}"),
        resource_type: "tools".into(),
        items,
        total_count: Some(tools.len()),
        truncated: tools.len() > limit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_bridge_unavailable_when_empty_url() {
        let provider = McpBridgeProvider::new("test", "");
        assert!(!provider.is_available());
    }

    #[test]
    fn mcp_bridge_available_with_url() {
        let provider = McpBridgeProvider::new("kb", "http://localhost:8080");
        assert!(provider.is_available());
        assert_eq!(provider.id(), "mcp_bridge");
    }

    #[test]
    fn mcp_bridge_supported_actions() {
        let provider = McpBridgeProvider::new("test", "http://localhost");
        assert!(provider.supported_actions().contains(&"resources"));
        assert!(provider.supported_actions().contains(&"tools"));
    }
}
