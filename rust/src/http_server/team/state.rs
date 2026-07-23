#[allow(clippy::wildcard_imports)]
use super::*;

#[derive(Clone)]
pub(super) struct TeamAuthContext {
    pub(super) token_id: String,
    pub(super) scopes: BTreeSet<TeamScope>,
}

#[derive(Clone)]
pub struct TeamRequestContext {
    pub workspace_id: String,
}

#[derive(Clone)]
pub struct TeamState {
    pub(super) auth: Arc<Vec<TeamTokenConfig>>,
    pub(super) engine: Arc<TeamContextEngine>,
    pub(super) audit: Arc<tokio::sync::Mutex<tokio::fs::File>>,
    pub savings_store_dir: Arc<tokio::sync::Mutex<std::path::PathBuf>>,
    /// Measurement roots for the billing-plane storage report (GL #463).
    pub storage_roots: crate::http_server::team_billing::StorageRoots,
    /// 60 s cache for the measured storage report.
    pub storage_cache: Arc<tokio::sync::Mutex<crate::http_server::team_billing::StorageCache>>,
    /// Configured managed connectors (#281), secret-bearing — never serialized
    /// back out; [`connectors::v1_connectors`] exposes a secret-free view.
    pub connectors: Arc<Vec<connectors::ConnectorConfig>>,
    /// Directory holding each connector's persisted run state (one file per id).
    pub connectors_state_dir: Arc<std::path::PathBuf>,
}

#[derive(Clone)]
pub struct TeamAppState {
    pub(super) concurrency: Arc<tokio::sync::Semaphore>,
    pub(super) rate: Arc<crate::http_server::RateLimiter>,
    pub(super) timeout: Duration,
    pub team: Arc<TeamState>,
    pub(super) max_body_bytes: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ToolCallBody {
    pub(super) name: String,
    #[serde(default)]
    pub(super) arguments: Option<Value>,
    #[serde(default)]
    pub(super) workspace_id: Option<String>,
    #[serde(default)]
    pub(super) channel_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ToolsQuery {
    #[serde(default)]
    pub(super) offset: Option<usize>,
    #[serde(default)]
    pub(super) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EventsQuery {
    #[serde(default)]
    pub(super) since: Option<i64>,
    #[serde(default)]
    pub(super) limit: Option<usize>,
    #[serde(default)]
    pub(super) channel_id: Option<String>,
}
