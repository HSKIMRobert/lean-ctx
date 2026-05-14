use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Anthropic,
    OpenAi,
    Gemini,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestBreakdown {
    pub provider: Provider,
    pub model: String,
    pub system_prompt_tokens: usize,
    pub user_message_tokens: usize,
    pub assistant_message_tokens: usize,
    pub tool_definition_tokens: usize,
    pub tool_definition_count: usize,
    pub tool_result_tokens: usize,
    pub image_count: usize,
    pub total_input_tokens: usize,
    pub message_count: usize,
}

pub fn analyze_request(body: &Value, provider: Provider) -> RequestBreakdown {
    match provider {
        Provider::Anthropic => analyze_anthropic(body),
        Provider::OpenAi => analyze_openai(body),
        Provider::Gemini => analyze_gemini(body),
    }
}

fn analyze_anthropic(body: &Value) -> RequestBreakdown {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown")
        .to_string();

    let system_prompt_tokens = match body.get("system") {
        Some(Value::String(s)) => chars_to_tokens(s.len()),
        Some(Value::Array(arr)) => {
            arr.iter()
                .map(|block| {
                    block
                        .get("text")
                        .and_then(|t| t.as_str())
                        .map_or(0, str::len)
                })
                .sum::<usize>()
                / 4
        }
        _ => 0,
    };

    let tool_definition_tokens = body
        .get("tools")
        .and_then(|t| t.as_array())
        .map_or(0, |arr| json_chars(arr) / 4);

    let tool_definition_count = body
        .get("tools")
        .and_then(|t| t.as_array())
        .map_or(0, Vec::len);

    let mut user_message_tokens = 0;
    let mut assistant_message_tokens = 0;
    let mut tool_result_tokens = 0;
    let mut image_count = 0;
    let mut message_count = 0;

    if let Some(messages) = body.get("messages").and_then(|m| m.as_array()) {
        message_count = messages.len();
        for msg in messages {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
            let content_tokens = estimate_content_tokens(msg.get("content"));
            let has_images = count_images(msg.get("content"));
            image_count += has_images;

            match role {
                "user" => {
                    if has_tool_results(msg.get("content")) {
                        tool_result_tokens += content_tokens;
                    } else {
                        user_message_tokens += content_tokens;
                    }
                }
                "assistant" => assistant_message_tokens += content_tokens,
                _ => user_message_tokens += content_tokens,
            }
        }
    }

    let total_input_tokens = system_prompt_tokens
        + user_message_tokens
        + assistant_message_tokens
        + tool_definition_tokens
        + tool_result_tokens;

    RequestBreakdown {
        provider: Provider::Anthropic,
        model,
        system_prompt_tokens,
        user_message_tokens,
        assistant_message_tokens,
        tool_definition_tokens,
        tool_definition_count,
        tool_result_tokens,
        image_count,
        total_input_tokens,
        message_count,
    }
}

fn analyze_openai(body: &Value) -> RequestBreakdown {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown")
        .to_string();

    let mut system_prompt_tokens = 0;
    let mut user_message_tokens = 0;
    let mut assistant_message_tokens = 0;
    let mut tool_result_tokens = 0;
    let mut image_count = 0;
    let mut message_count = 0;

    if let Some(messages) = body.get("messages").and_then(|m| m.as_array()) {
        message_count = messages.len();
        for msg in messages {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
            let content_tokens = estimate_content_tokens(msg.get("content"));
            image_count += count_images(msg.get("content"));

            match role {
                "system" | "developer" => system_prompt_tokens += content_tokens,
                "assistant" => assistant_message_tokens += content_tokens,
                "tool" => tool_result_tokens += content_tokens,
                _ => user_message_tokens += content_tokens,
            }
        }
    }

    let tool_definition_tokens = body
        .get("tools")
        .and_then(|t| t.as_array())
        .map_or(0, |arr| json_chars(arr) / 4);

    let tool_definition_count = body
        .get("tools")
        .and_then(|t| t.as_array())
        .map_or(0, Vec::len);

    let total_input_tokens = system_prompt_tokens
        + user_message_tokens
        + assistant_message_tokens
        + tool_definition_tokens
        + tool_result_tokens;

    RequestBreakdown {
        provider: Provider::OpenAi,
        model,
        system_prompt_tokens,
        user_message_tokens,
        assistant_message_tokens,
        tool_definition_tokens,
        tool_definition_count,
        tool_result_tokens,
        image_count,
        total_input_tokens,
        message_count,
    }
}

fn analyze_gemini(body: &Value) -> RequestBreakdown {
    let model = "gemini".to_string();

    let system_prompt_tokens = body
        .get("systemInstruction")
        .and_then(|si| si.get("parts"))
        .and_then(|p| p.as_array())
        .map_or(0, |parts| {
            parts
                .iter()
                .map(|p| p.get("text").and_then(|t| t.as_str()).map_or(0, str::len))
                .sum::<usize>()
                / 4
        });

    let mut user_message_tokens = 0;
    let mut assistant_message_tokens = 0;
    let mut tool_result_tokens = 0;
    let mut message_count = 0;

    if let Some(contents) = body.get("contents").and_then(|c| c.as_array()) {
        message_count = contents.len();
        for content in contents {
            let role = content
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("user");
            let parts_tokens = content
                .get("parts")
                .and_then(|p| p.as_array())
                .map_or(0, |parts| {
                    parts
                        .iter()
                        .map(|p| {
                            if p.get("functionResponse").is_some() {
                                json_chars(std::slice::from_ref(p)) / 4
                            } else {
                                p.get("text")
                                    .and_then(|t| t.as_str())
                                    .map_or(0, |s| chars_to_tokens(s.len()))
                            }
                        })
                        .sum::<usize>()
                });

            let has_fn_response = content
                .get("parts")
                .and_then(|p| p.as_array())
                .is_some_and(|parts| parts.iter().any(|p| p.get("functionResponse").is_some()));

            if has_fn_response {
                tool_result_tokens += parts_tokens;
            } else {
                match role {
                    "model" => assistant_message_tokens += parts_tokens,
                    _ => user_message_tokens += parts_tokens,
                }
            }
        }
    }

    let tool_definition_tokens = body
        .get("tools")
        .and_then(|t| t.as_array())
        .map_or(0, |arr| json_chars(arr) / 4);

    let tool_definition_count = body
        .get("tools")
        .and_then(|t| t.as_array())
        .map_or(0, |arr| {
            arr.iter()
                .filter_map(|t| t.get("functionDeclarations").and_then(|f| f.as_array()))
                .map(Vec::len)
                .sum()
        });

    let total_input_tokens = system_prompt_tokens
        + user_message_tokens
        + assistant_message_tokens
        + tool_definition_tokens
        + tool_result_tokens;

    RequestBreakdown {
        provider: Provider::Gemini,
        model,
        system_prompt_tokens,
        user_message_tokens,
        assistant_message_tokens,
        tool_definition_tokens,
        tool_definition_count,
        tool_result_tokens,
        image_count: 0,
        total_input_tokens,
        message_count,
    }
}

fn chars_to_tokens(chars: usize) -> usize {
    chars / 4
}

fn json_chars(arr: &[Value]) -> usize {
    arr.iter().map(|v| v.to_string().len()).sum()
}

fn estimate_content_tokens(content: Option<&Value>) -> usize {
    match content {
        Some(Value::String(s)) => chars_to_tokens(s.len()),
        Some(Value::Array(arr)) => arr
            .iter()
            .map(|block| {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    chars_to_tokens(text.len())
                } else {
                    block.to_string().len() / 4
                }
            })
            .sum(),
        Some(v) => v.to_string().len() / 4,
        None => 0,
    }
}

fn count_images(content: Option<&Value>) -> usize {
    match content {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter(|block| {
                block.get("type").and_then(|t| t.as_str()) == Some("image")
                    || block.get("type").and_then(|t| t.as_str()) == Some("image_url")
            })
            .count(),
        _ => 0,
    }
}

fn has_tool_results(content: Option<&Value>) -> bool {
    match content {
        Some(Value::Array(arr)) => arr
            .iter()
            .any(|block| block.get("type").and_then(|t| t.as_str()) == Some("tool_result")),
        _ => false,
    }
}

// Shared state for introspection data
pub struct IntrospectState {
    pub last_breakdown: std::sync::Mutex<Option<RequestBreakdown>>,
    pub total_system_prompt_tokens: AtomicU64,
    pub total_requests: AtomicU64,
}

impl Default for IntrospectState {
    fn default() -> Self {
        Self {
            last_breakdown: std::sync::Mutex::new(None),
            total_system_prompt_tokens: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
        }
    }
}

impl IntrospectState {
    pub fn record(&self, breakdown: RequestBreakdown) {
        self.total_system_prompt_tokens
            .fetch_add(breakdown.system_prompt_tokens as u64, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut last) = self.last_breakdown.lock() {
            *last = Some(breakdown);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_basic() {
        let body = serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "system": "You are a helpful assistant.",
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": "Hi there!"}
            ],
            "tools": [{"name": "read", "description": "Read a file", "input_schema": {}}]
        });
        let b = analyze_request(&body, Provider::Anthropic);
        assert_eq!(b.provider, Provider::Anthropic);
        assert!(b.system_prompt_tokens > 0);
        assert_eq!(b.message_count, 2);
        assert!(b.user_message_tokens > 0);
        assert!(b.assistant_message_tokens > 0);
        assert_eq!(b.tool_definition_count, 1);
        assert!(b.tool_definition_tokens > 0);
    }

    #[test]
    fn openai_system_message() {
        let body = serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "System prompt here"},
                {"role": "user", "content": "Hello"},
                {"role": "tool", "content": "tool result data", "tool_call_id": "x"}
            ]
        });
        let b = analyze_request(&body, Provider::OpenAi);
        assert!(b.system_prompt_tokens > 0);
        assert!(b.user_message_tokens > 0);
        assert!(b.tool_result_tokens > 0);
        assert_eq!(b.message_count, 3);
    }

    #[test]
    fn gemini_system_instruction() {
        let body = serde_json::json!({
            "systemInstruction": {
                "parts": [{"text": "Be concise and helpful to the user at all times."}]
            },
            "contents": [
                {"role": "user", "parts": [{"text": "What is the meaning of life and everything?"}]},
                {"role": "model", "parts": [{"text": "The answer is 42 according to Douglas Adams."}]}
            ]
        });
        let b = analyze_request(&body, Provider::Gemini);
        assert!(b.system_prompt_tokens > 0);
        assert!(b.user_message_tokens > 0);
        assert!(b.assistant_message_tokens > 0);
        assert_eq!(b.message_count, 2);
    }
}
