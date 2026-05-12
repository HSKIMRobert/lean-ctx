//! API proxy upstream overrides (`config.toml`).

use serde::{Deserialize, Serialize};

/// API proxy upstream overrides. `None` = use provider default.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxyConfig {
    pub anthropic_upstream: Option<String>,
    pub openai_upstream: Option<String>,
    pub gemini_upstream: Option<String>,
}

impl ProxyConfig {
    pub fn resolve_upstream(&self, provider: ProxyProvider) -> String {
        let (env_var, config_val, default) = match provider {
            ProxyProvider::Anthropic => (
                "LEAN_CTX_ANTHROPIC_UPSTREAM",
                self.anthropic_upstream.as_deref(),
                "https://api.anthropic.com",
            ),
            ProxyProvider::OpenAi => (
                "LEAN_CTX_OPENAI_UPSTREAM",
                self.openai_upstream.as_deref(),
                "https://api.openai.com",
            ),
            ProxyProvider::Gemini => (
                "LEAN_CTX_GEMINI_UPSTREAM",
                self.gemini_upstream.as_deref(),
                "https://generativelanguage.googleapis.com",
            ),
        };
        let resolved = std::env::var(env_var)
            .ok()
            .and_then(|v| normalize_url_opt(&v))
            .or_else(|| config_val.and_then(normalize_url_opt))
            .unwrap_or_else(|| normalize_url(default));
        match validate_upstream_url(&resolved) {
            Ok(url) => url,
            Err(e) => {
                tracing::warn!("upstream validation failed, using default: {e}");
                normalize_url(default)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProxyProvider {
    Anthropic,
    OpenAi,
    Gemini,
}

pub fn normalize_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

pub fn normalize_url_opt(value: &str) -> Option<String> {
    let trimmed = normalize_url(value);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

const ALLOWED_UPSTREAM_HOSTS: &[&str] = &[
    "api.anthropic.com",
    "api.openai.com",
    "generativelanguage.googleapis.com",
];

pub(super) fn validate_upstream_url(url: &str) -> Result<String, String> {
    let normalized = normalize_url(url);
    if is_local_proxy_url(&normalized) {
        return Ok(normalized);
    }
    if !normalized.starts_with("https://") {
        return Err(format!(
            "upstream URL must use HTTPS: {normalized} (set LEAN_CTX_ALLOW_CUSTOM_UPSTREAM=1 to override)"
        ));
    }
    let host = normalized
        .strip_prefix("https://")
        .unwrap_or(&normalized)
        .split('/')
        .next()
        .unwrap_or("");
    let host_no_port = host.split(':').next().unwrap_or(host);
    if ALLOWED_UPSTREAM_HOSTS.contains(&host_no_port)
        || std::env::var("LEAN_CTX_ALLOW_CUSTOM_UPSTREAM").is_ok()
    {
        Ok(normalized)
    } else {
        Err(format!(
            "upstream host '{host_no_port}' not in allowlist {ALLOWED_UPSTREAM_HOSTS:?} (set LEAN_CTX_ALLOW_CUSTOM_UPSTREAM=1 to override)"
        ))
    }
}

pub fn is_local_proxy_url(value: &str) -> bool {
    let n = normalize_url(value);
    n.starts_with("http://127.0.0.1:")
        || n.starts_with("http://localhost:")
        || n.starts_with("http://[::1]:")
}
