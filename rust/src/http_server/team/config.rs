#[allow(clippy::wildcard_imports)]
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamServerConfig {
    pub host: String,
    pub port: u16,
    pub default_workspace_id: String,
    pub workspaces: Vec<TeamWorkspaceConfig>,
    #[serde(default)]
    pub tokens: Vec<TeamTokenConfig>,
    pub audit_log_path: PathBuf,
    #[serde(default)]
    pub disable_host_check: bool,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: usize,
    #[serde(default = "default_max_concurrency")]
    pub max_concurrency: usize,
    #[serde(default = "default_max_rps")]
    pub max_rps: u32,
    #[serde(default = "default_rate_burst")]
    pub rate_burst: u32,
    #[serde(default = "default_request_timeout_ms")]
    pub request_timeout_ms: u64,
    #[serde(default)]
    pub stateful_mode: bool,
    #[serde(default = "default_true")]
    pub json_response: bool,
    /// Hosted-storage quota in bytes (`storageQuotaBytes` in `team.json`),
    /// rendered per plan by the control plane's provisioning bridge (#282).
    /// Omitted ⇒ the server defaults to the Team tier's 5 GiB; the
    /// `LEANCTX_TEAM_STORAGE_QUOTA_BYTES` env var overrides both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_quota_bytes: Option<u64>,
    /// Slack/Discord/generic webhook for the weekly team-ROI summary
    /// (`roiWebhookUrl` in `team.json`, GL #388). HTTPS only — the server
    /// refuses to start with a plaintext URL. Omitted ⇒ no webhook posts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roi_webhook_url: Option<String>,
    /// Managed connectors (#281): scheduled hosted source syncs, rendered into
    /// `team.json` by the control plane (which enforces the `managed_connectors`
    /// entitlement count and encrypts each `secret` at rest). Omitted ⇒ none.
    #[serde(default)]
    pub connectors: Vec<connectors::ConnectorConfig>,
}

fn default_true() -> bool {
    true
}
fn default_max_body_bytes() -> usize {
    2 * 1024 * 1024
}
fn default_max_concurrency() -> usize {
    32
}
fn default_max_rps() -> u32 {
    50
}
fn default_rate_burst() -> u32 {
    100
}
fn default_request_timeout_ms() -> u64 {
    30_000
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamWorkspaceConfig {
    pub id: String,
    pub label: Option<String>,
    pub root: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamTokenConfig {
    pub id: String,
    /// Stored as lowercase hex of SHA-256(token).
    pub sha256_hex: String,
    /// Explicitly granted scopes. May be empty when a [`role`](Self::role) is set.
    #[serde(default)]
    pub scopes: Vec<TeamScope>,
    /// Optional RBAC role (EPIC 13.2). Expands to a scope set that is unioned
    /// with `scopes`. Lets admins grant `viewer`/`member`/`admin`/`owner`
    /// instead of hand-picking scopes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<roles::TeamRole>,
}

impl TeamTokenConfig {
    /// The effective scopes for this token: explicit scopes ∪ role-derived
    /// scopes. This is what authorization is evaluated against (EPIC 13.2).
    #[must_use]
    pub fn effective_scopes(&self) -> BTreeSet<TeamScope> {
        let mut s: BTreeSet<TeamScope> = self.scopes.iter().copied().collect();
        if let Some(role) = self.role {
            s.extend(role.scopes());
        }
        s
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamScope {
    Search,
    Graph,
    Artifacts,
    Index,
    Events,
    SessionMutations,
    Knowledge,
    Audit,
}

impl TeamScope {
    /// Every scope, used by role expansion (EPIC 13.2) to grant full access.
    #[must_use]
    pub fn all() -> &'static [TeamScope] {
        &[
            TeamScope::Search,
            TeamScope::Graph,
            TeamScope::Artifacts,
            TeamScope::Index,
            TeamScope::Events,
            TeamScope::SessionMutations,
            TeamScope::Knowledge,
            TeamScope::Audit,
        ]
    }
}

impl TeamServerConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let s =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let cfg: Self =
            serde_json::from_str(&s).with_context(|| format!("parse {}", path.display()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let s = serde_json::to_string_pretty(self).context("serialize TeamServerConfig")?;
        std::fs::write(path, format!("{s}\n")).with_context(|| format!("write {}", path.display()))
    }

    pub fn validate(&self) -> Result<()> {
        if self.workspaces.is_empty() {
            return Err(anyhow!("team server requires at least 1 workspace"));
        }
        let mut ws_ids = BTreeSet::new();
        for ws in &self.workspaces {
            let id = ws.id.trim();
            if id.is_empty() {
                return Err(anyhow!("workspace id must be non-empty"));
            }
            if !ws_ids.insert(id.to_string()) {
                return Err(anyhow!("duplicate workspace id: {id}"));
            }
            if !ws.root.exists() {
                return Err(anyhow!(
                    "workspace root does not exist: {}",
                    ws.root.display()
                ));
            }
        }
        if !ws_ids.contains(self.default_workspace_id.trim()) {
            return Err(anyhow!(
                "defaultWorkspaceId '{}' not found in workspaces",
                self.default_workspace_id
            ));
        }

        let mut token_ids = BTreeSet::new();
        for t in &self.tokens {
            let id = t.id.trim();
            if id.is_empty() {
                return Err(anyhow!("token id must be non-empty"));
            }
            if !token_ids.insert(id.to_string()) {
                return Err(anyhow!("duplicate token id: {id}"));
            }
            // A token must grant access via explicit scopes and/or a role
            // (EPIC 13.2). An empty effective scope set is a misconfiguration.
            if t.effective_scopes().is_empty() {
                return Err(anyhow!("token '{id}' must have at least 1 scope or a role"));
            }
            parse_sha256_hex(&t.sha256_hex)
                .with_context(|| format!("token '{id}' invalid sha256Hex"))?;
        }

        if let Some(parent) = self.audit_log_path.parent()
            && !parent.as_os_str().is_empty()
            && !parent.exists()
        {
            return Err(anyhow!(
                "auditLogPath parent does not exist: {}",
                parent.display()
            ));
        }
        Ok(())
    }

    pub fn validate_for_serve(&self) -> Result<()> {
        self.validate()?;
        if self.tokens.is_empty() {
            return Err(anyhow!("team server requires at least 1 token"));
        }
        Ok(())
    }
}
