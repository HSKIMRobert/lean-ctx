//! LLM client for benchmark tasks.
//!
//! Supports both Anthropic Messages API and OpenAI Chat Completions formats.
//! All traffic goes through the lean-ctx proxy which handles auth, compression,
//! and routing transparently.

use std::time::{Duration, Instant};

/// Response from a single LLM completion.
#[derive(Debug, Clone)]
pub(crate) struct CompletionResponse {
    pub content: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model_used: String,
    pub latency_ms: u64,
}

/// API format for the upstream.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ApiFormat {
    Anthropic,
    OpenAi,
}

/// Configuration for LLM calls.
#[derive(Debug, Clone)]
pub(crate) struct LlmClientConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub timeout: Duration,
    pub format: ApiFormat,
}

impl LlmClientConfig {
    /// Through lean-ctx proxy with OpenAI format (default for ChatGPT users).
    pub(crate) fn via_proxy_openai(model: &str) -> Self {
        let proxy_port = std::env::var("LEAN_CTX_PROXY_PORT").unwrap_or_else(|_| "4444".into());
        let token = crate::core::session_token::resolve_proxy_token("LEAN_CTX_PROXY_TOKEN");
        Self {
            base_url: format!("http://127.0.0.1:{proxy_port}"),
            api_key: token,
            model: model.into(),
            max_tokens: 4096,
            timeout: Duration::from_mins(2),
            format: ApiFormat::OpenAi,
        }
    }

    /// Direct Anthropic API (requires ANTHROPIC_API_KEY).
    pub(crate) fn direct_anthropic(model: &str) -> Result<Self, String> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;
        Ok(Self {
            base_url: "https://api.anthropic.com".into(),
            api_key,
            model: model.into(),
            max_tokens: 4096,
            timeout: Duration::from_mins(2),
            format: ApiFormat::Anthropic,
        })
    }

    /// Through lean-ctx proxy with Anthropic format (requires ANTHROPIC_API_KEY).
    pub(crate) fn via_proxy_anthropic(model: &str) -> Result<Self, String> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;
        let proxy_port = std::env::var("LEAN_CTX_PROXY_PORT").unwrap_or_else(|_| "4444".into());
        Ok(Self {
            base_url: format!("http://127.0.0.1:{proxy_port}"),
            api_key,
            model: model.into(),
            max_tokens: 4096,
            timeout: Duration::from_mins(2),
            format: ApiFormat::Anthropic,
        })
    }
}

/// Call the LLM and return the completion (dispatches by format).
pub(crate) fn complete(
    config: &LlmClientConfig,
    system: &str,
    prompt: &str,
) -> Result<CompletionResponse, String> {
    match config.format {
        ApiFormat::Anthropic => complete_anthropic(config, system, prompt),
        ApiFormat::OpenAi => complete_openai(config, system, prompt),
    }
}

fn build_agent(config: &LlmClientConfig) -> ureq::Agent {
    crate::core::http_client::ureq_agent(
        ureq::config::Config::builder()
            .tls_config(crate::core::http_client::platform_tls_config())
            .timeout_global(Some(config.timeout))
            .build(),
    )
}

fn complete_anthropic(
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

    let agent = build_agent(config);
    let payload = serde_json::to_vec(&body).map_err(|e| format!("json: {e}"))?;
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
        .map_err(|e| format!("read: {e}"))?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("parse: {e}"))?;

    let content = json
        .pointer("/content/0/text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("no content in response: {text}"))?
        .to_string();

    let usage = &json["usage"];
    Ok(CompletionResponse {
        content,
        input_tokens: usage["input_tokens"].as_u64().unwrap_or(0),
        output_tokens: usage["output_tokens"].as_u64().unwrap_or(0),
        model_used: json["model"].as_str().unwrap_or(&config.model).to_string(),
        latency_ms,
    })
}

fn complete_openai(
    config: &LlmClientConfig,
    system: &str,
    prompt: &str,
) -> Result<CompletionResponse, String> {
    let url = format!("{}/v1/chat/completions", config.base_url);

    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(serde_json::json!({"role": "system", "content": system}));
    }
    messages.push(serde_json::json!({"role": "user", "content": prompt}));

    let body = serde_json::json!({
        "model": config.model,
        "max_tokens": config.max_tokens,
        "messages": messages,
    });

    let agent = build_agent(config);
    let payload = serde_json::to_vec(&body).map_err(|e| format!("json: {e}"))?;
    let start = Instant::now();

    let resp = agent
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", config.api_key))
        .send(payload.as_slice())
        .map_err(|e| format!("API call failed: {e}"))?;

    let latency_ms = start.elapsed().as_millis() as u64;
    let text = resp
        .into_body()
        .read_to_string()
        .map_err(|e| format!("read: {e}"))?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("parse: {e}"))?;

    let content = json
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("no content in response: {text}"))?
        .to_string();

    let usage = &json["usage"];
    Ok(CompletionResponse {
        content,
        input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
        output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
        model_used: json["model"].as_str().unwrap_or(&config.model).to_string(),
        latency_ms,
    })
}

/// Pricing per million tokens (input/output) for known models.
pub(crate) fn cost_for_tokens(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
    let (input_per_m, output_per_m) = match model {
        m if m.contains("haiku") => (0.25, 1.25),
        m if m.contains("sonnet") => (3.0, 15.0),
        m if m.contains("opus") => (15.0, 75.0),
        m if m.contains("gpt-4") => (2.50, 10.0),
        m if m.contains("gpt-5") => (2.50, 10.0),
        m if m.contains("o3") || m.contains("o4") => (2.50, 10.0),
        _ => (3.0, 15.0),
    };
    (input_tokens as f64 * input_per_m + output_tokens as f64 * output_per_m) / 1_000_000.0
}

/// Extract Python code from an LLM response.
pub(crate) fn extract_code(response: &str) -> String {
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
    fn cost_gpt() {
        let cost = cost_for_tokens("gpt-5.6-terra", 1_000_000, 1_000_000);
        assert!((cost - 12.5).abs() < 0.01);
    }

    #[test]
    fn cost_haiku() {
        let cost = cost_for_tokens("claude-haiku-4-5-20250514", 1_000_000, 0);
        assert!((cost - 0.25).abs() < 0.01);
    }

    #[test]
    fn proxy_openai_config_no_key_needed() {
        let cfg = LlmClientConfig::via_proxy_openai("gpt-5.6-terra");
        assert_eq!(cfg.format, ApiFormat::OpenAi);
        assert!(cfg.base_url.contains("127.0.0.1"));
    }
}
