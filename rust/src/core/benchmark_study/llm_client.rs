//! Synchronous LLM client for benchmark tasks.
//!
//! Calls the Anthropic Messages API (directly or through the lean-ctx proxy).
//! For CompressOnly/Combined arms, traffic goes through the proxy which applies
//! compression and/or routing transparently.

use std::time::{Duration, Instant};



/// Response from a single LLM completion.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model_used: String,
    pub latency_ms: u64,
}

/// Configuration for LLM calls.
#[derive(Debug, Clone)]
pub struct LlmClientConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub timeout: Duration,
}

impl LlmClientConfig {
    /// Direct Anthropic API (Control + RouteOnly arms).
    pub fn direct(model: &str) -> Result<Self, String> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;
        Ok(Self {
            base_url: "https://api.anthropic.com".into(),
            api_key,
            model: model.into(),
            max_tokens: 4096,
            timeout: Duration::from_mins(2),
        })
    }

    /// Through lean-ctx proxy (CompressOnly + Combined arms).
    pub fn via_proxy(model: &str) -> Result<Self, String> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;
        let proxy_port = std::env::var("LEAN_CTX_PROXY_PORT").unwrap_or_else(|_| "9018".into());
        Ok(Self {
            base_url: format!("http://127.0.0.1:{proxy_port}"),
            api_key,
            model: model.into(),
            max_tokens: 4096,
            timeout: Duration::from_mins(2),
        })
    }
}

/// Call the Anthropic Messages API and return the completion.
pub fn complete(
    config: &LlmClientConfig,
    system: &str,
    prompt: &str,
) -> Result<CompletionResponse, String> {
    let url = format!("{}/v1/messages", config.base_url);

    let messages = vec![serde_json::json!({"role": "user", "content": prompt})];

    let body = if system.is_empty() {
        serde_json::json!({
            "model": config.model,
            "max_tokens": config.max_tokens,
            "messages": messages,
        })
    } else {
        serde_json::json!({
            "model": config.model,
            "max_tokens": config.max_tokens,
            "system": system,
            "messages": messages,
        })
    };

    let agent = crate::core::http_client::ureq_agent(
        ureq::config::Config::builder()
            .tls_config(crate::core::http_client::platform_tls_config())
            .timeout_global(Some(config.timeout))
            .build(),
    );

    let payload = serde_json::to_vec(&body).map_err(|e| format!("json serialize: {e}"))?;
    let start = Instant::now();

    let resp = agent
        .post(&url)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .send(payload.as_slice())
        .map_err(|e| format!("API call failed: {e}"))?;

    let latency_ms = start.elapsed().as_millis() as u64;

    let text = resp
        .into_body()
        .read_to_string()
        .map_err(|e| format!("read response: {e}"))?;

    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("parse response: {e}"))?;

    let content = json
        .pointer("/content/0/text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("no content in response: {text}"))?
        .to_string();

    let usage = &json["usage"];
    let input_tokens = usage["input_tokens"].as_u64().unwrap_or(0);
    let output_tokens = usage["output_tokens"].as_u64().unwrap_or(0);
    let model_used = json["model"].as_str().unwrap_or(&config.model).to_string();

    Ok(CompletionResponse {
        content,
        input_tokens,
        output_tokens,
        model_used,
        latency_ms,
    })
}

/// Pricing per million tokens (input/output) for known models.
pub fn cost_for_tokens(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
    let (input_per_m, output_per_m) = match model {
        m if m.contains("haiku") => (0.25, 1.25),
        m if m.contains("sonnet") => (3.0, 15.0),
        m if m.contains("opus") => (15.0, 75.0),
        _ => (3.0, 15.0),
    };
    (input_tokens as f64 * input_per_m + output_tokens as f64 * output_per_m) / 1_000_000.0
}

/// Extract Python code from an LLM response.
///
/// Handles common patterns: raw code, ```python blocks, ```blocks.
pub fn extract_code(response: &str) -> String {
    if let Some(start) = response.find("```python") {
        let code_start = start + "```python".len();
        if let Some(end) = response[code_start..].find("```") {
            return response[code_start..code_start + end].trim().to_string();
        }
    }

    if let Some(start) = response.find("```") {
        let code_start = start + 3;
        let after_lang = response[code_start..]
            .find('\n')
            .map_or(code_start, |p| code_start + p + 1);
        if let Some(end) = response[after_lang..].find("```") {
            return response[after_lang..after_lang + end].trim().to_string();
        }
    }

    response.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_code_from_python_block() {
        let resp = "Here's the solution:\n```python\ndef add(a, b):\n    return a + b\n```\nDone!";
        assert_eq!(extract_code(resp), "def add(a, b):\n    return a + b");
    }

    #[test]
    fn extract_code_from_bare_block() {
        let resp = "```\ndef add(a, b):\n    return a + b\n```";
        assert_eq!(extract_code(resp), "def add(a, b):\n    return a + b");
    }

    #[test]
    fn extract_code_raw() {
        let resp = "def add(a, b):\n    return a + b";
        assert_eq!(extract_code(resp), "def add(a, b):\n    return a + b");
    }

    #[test]
    fn cost_calculation() {
        let cost = cost_for_tokens("claude-sonnet-4-20250514", 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[test]
    fn cost_haiku() {
        let cost = cost_for_tokens("claude-haiku-4-5-20250514", 1_000_000, 0);
        assert!((cost - 0.25).abs() < 0.01);
    }
}
