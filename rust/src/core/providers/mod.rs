pub mod cache;
pub mod config;
pub mod gitlab;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResult {
    pub provider: String,
    pub resource_type: String,
    pub items: Vec<ProviderItem>,
    pub total_count: Option<usize>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
        out
    }
}
