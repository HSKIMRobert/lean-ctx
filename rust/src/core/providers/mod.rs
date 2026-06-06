pub mod cache;
pub mod config;
pub mod config_provider;
pub mod github;
pub mod gitlab;
pub mod init;
pub mod jira;
pub mod jira_oauth;
pub mod mcp_bridge;
pub mod postgres;
pub mod provider_trait;
pub mod registry;

pub use provider_trait::{ContextPacket, ContextProvider, ProviderParams};
pub use registry::{global_registry, ProviderRegistry};

use serde::{Deserialize, Serialize};

use crate::core::evidence::Claim;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResult {
    pub provider: String,
    pub resource_type: String,
    pub items: Vec<ProviderItem>,
    pub total_count: Option<usize>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderItem {
    pub id: String,
    pub title: String,
    pub state: Option<String>,
    pub author: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub url: Option<String>,
    pub labels: Vec<String>,
    pub body: Option<String>,
    /// Attributable evidence distilled from this item (confidence + source).
    /// Empty for plain records; populated by research/extraction providers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<Claim>,
}

impl ProviderResult {
    pub fn format_compact(&self) -> String {
        let mut out = format!(
            "{} {} ({}{}):\n",
            self.provider,
            self.resource_type,
            self.items.len(),
            if self.truncated { "+" } else { "" }
        );
        for item in &self.items {
            let state = item.state.as_deref().unwrap_or("");
            let labels = if item.labels.is_empty() {
                String::new()
            } else {
                format!(" [{}]", item.labels.join(","))
            };
            out.push_str(&format!(
                "  #{} {} ({}){}\n",
                item.id, item.title, state, labels
            ));
            for claim in &item.claims {
                out.push_str("    ▸ ");
                out.push_str(&claim.render());
                out.push('\n');
            }
        }
        out
    }
}
