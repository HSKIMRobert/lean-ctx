use serde_json::{Value, json};

use super::{NormalizedRequest, WebAppProvider};

pub(crate) fn normalize_request(
    provider: WebAppProvider,
    body: &[u8],
) -> Option<NormalizedRequest> {
    match provider {
        WebAppProvider::ClaudeWeb => normalize_claude_web(body),
        WebAppProvider::ChatGptWeb => normalize_chatgpt_web(body),
        WebAppProvider::GeminiWeb => normalize_gemini_web(body),
    }
}

fn normalize_claude_web(body: &[u8]) -> Option<NormalizedRequest> {
    let body: Value = serde_json::from_slice(body).ok()?;
    let messages = body.get("messages")?.as_array()?.clone();
    let system_prompt = optional_string(&body, "system");
    let model = optional_string(&body, "model");

    Some(NormalizedRequest {
        provider: WebAppProvider::ClaudeWeb,
        messages,
        system_prompt,
        model,
        conversation_id: None,
        parent_message_id: None,
    })
}

fn normalize_chatgpt_web(body: &[u8]) -> Option<NormalizedRequest> {
    let body: Value = serde_json::from_slice(body).ok()?;
    let messages = body
        .get("messages")?
        .as_array()?
        .iter()
        .map(normalize_chatgpt_message)
        .collect::<Option<Vec<_>>>()?;
    let system_prompt = None;
    let model = optional_string(&body, "model");
    let conversation_id = optional_string(&body, "conversation_id");
    let parent_message_id = optional_string(&body, "parent_message_id");

    Some(NormalizedRequest {
        provider: WebAppProvider::ChatGptWeb,
        messages,
        system_prompt,
        model,
        conversation_id,
        parent_message_id,
    })
}

fn normalize_chatgpt_message(message: &Value) -> Option<Value> {
    let role = message.get("author")?.get("role")?.as_str()?;
    let parts = message.get("content")?.get("parts")?.as_array()?;
    let content = parts
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()?
        .join("\n");

    Some(json!({"role": role, "content": content}))
}

fn normalize_gemini_web(body: &[u8]) -> Option<NormalizedRequest> {
    let body: Value = serde_json::from_slice(body).ok()?;
    let messages = body
        .get("contents")?
        .as_array()?
        .iter()
        .map(normalize_gemini_message)
        .collect::<Option<Vec<_>>>()?;
    let system_prompt = body
        .get("systemInstruction")
        .and_then(|instruction| instruction.get("parts"))
        .and_then(Value::as_array)
        .and_then(|parts| join_gemini_parts(parts));
    let model = body
        .get("generationConfig")
        .and_then(|config| config.get("model"))
        .and_then(Value::as_str)
        .map(str::to_owned);

    Some(NormalizedRequest {
        provider: WebAppProvider::GeminiWeb,
        messages,
        system_prompt,
        model,
        conversation_id: None,
        parent_message_id: None,
    })
}

fn normalize_gemini_message(message: &Value) -> Option<Value> {
    let role = match message.get("role")?.as_str()? {
        "model" => "assistant",
        role => role,
    };
    let parts = message.get("parts")?.as_array()?;
    let content = join_gemini_parts(parts)?;

    Some(json!({"role": role, "content": content}))
}

fn join_gemini_parts(parts: &[Value]) -> Option<String> {
    parts
        .iter()
        .map(|part| part.get("text")?.as_str())
        .collect::<Option<Vec<_>>>()
        .map(|texts| texts.join("\n"))
}

fn optional_string(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).map(str::to_owned)
}

#[allow(dead_code)]
fn estimate_tokens(messages: &[Value]) -> usize {
    messages
        .iter()
        .filter_map(|message| message.get("content"))
        .map(content_character_count)
        .sum::<usize>()
        / 4
}

fn content_character_count(content: &Value) -> usize {
    if let Some(text) = content.as_str() {
        return text.chars().count();
    }

    content.as_array().map_or(0, |parts| {
        parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .map(str::chars)
            .map(Iterator::count)
            .sum()
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{estimate_tokens, normalize_request};
    use crate::proxy::web_app::{WebAppProvider, detect_web_provider};

    #[test]
    fn test_detect_claude_web() {
        assert_eq!(
            detect_web_provider("claude.ai", "/api/organizations/org/chat_conversations"),
            Some(WebAppProvider::ClaudeWeb)
        );
    }

    #[test]
    fn test_detect_chatgpt_web() {
        assert_eq!(
            detect_web_provider("chatgpt.com", "/backend-api/conversation"),
            Some(WebAppProvider::ChatGptWeb)
        );
        assert_eq!(
            detect_web_provider("chat.openai.com", "/backend-api/conversation"),
            Some(WebAppProvider::ChatGptWeb)
        );
    }

    #[test]
    fn test_detect_gemini_web() {
        assert_eq!(
            detect_web_provider("gemini.google.com", "/v1beta/models/gemini:generateContent"),
            Some(WebAppProvider::GeminiWeb)
        );
    }

    #[test]
    fn test_detect_non_ai_host_returns_none() {
        assert_eq!(
            detect_web_provider("example.com", "/backend-api/conversation"),
            None
        );
    }

    #[test]
    fn test_normalize_claude_web_full_conversation() {
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 4096,
            "messages": [
                {"role": "user", "content": "Can you review the payment retry logic?"},
                {"role": "assistant", "content": "Yes. Share the retry policy and failure logs."},
                {"role": "user", "content": "Retries use exponential backoff capped at five minutes."},
                {"role": "assistant", "content": "The cap is sensible; add jitter to avoid synchronized retries."},
                {"role": "user", "content": "Show me a safe jitter calculation."}
            ],
            "system": "You are a senior reliability engineer."
        });
        let normalized = normalize_json(WebAppProvider::ClaudeWeb, &body);

        assert_eq!(normalized.provider, WebAppProvider::ClaudeWeb);
        assert_eq!(
            normalized.messages,
            body["messages"]
                .as_array()
                .expect("messages should be an array")
                .clone()
        );
        assert_eq!(normalized.messages.len(), 5);
        assert_eq!(normalized.model.as_deref(), Some("claude-sonnet-4-20250514"));
    }

    #[test]
    fn test_normalize_chatgpt_web_single_turn() {
        let body = json!({
            "action": "next",
            "messages": [{
                "id": "aaa27fdc-7e5d-49d8-a12a-c80491c2430f",
                "author": {"role": "user"},
                "content": {"content_type": "text", "parts": ["Summarize the incident timeline."]}
            }],
            "conversation_id": "86f73b54-c4c4-45aa-bf56-b31883c4d3df",
            "parent_message_id": "14588fa1-da2b-49a6-8c65-6de60c27cc3d",
            "model": "gpt-4o"
        });
        let normalized = normalize_json(WebAppProvider::ChatGptWeb, &body);

        assert_eq!(
            normalized.messages,
            vec![json!({"role": "user", "content": "Summarize the incident timeline."})]
        );
        assert_eq!(
            normalized.conversation_id.as_deref(),
            Some("86f73b54-c4c4-45aa-bf56-b31883c4d3df")
        );
        assert_eq!(
            normalized.parent_message_id.as_deref(),
            Some("14588fa1-da2b-49a6-8c65-6de60c27cc3d")
        );
        assert_eq!(normalized.model.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn test_normalize_gemini_web_parts_format() {
        let body = json!({
            "contents": [
                {"role": "user", "parts": [{"text": "Check the deployment status."}]},
                {"role": "model", "parts": [{"text": "The rollout is at 80%."}]},
                {"role": "user", "parts": [{"text": "List remaining regions."}, {"text": "Include ETA."}]}
            ],
            "systemInstruction": {"parts": [{"text": "Answer as an operations analyst."}]},
            "generationConfig": {"model": "gemini-2.0-flash"}
        });
        let normalized = normalize_json(WebAppProvider::GeminiWeb, &body);

        assert_eq!(normalized.messages[0]["role"], "user");
        assert_eq!(normalized.messages[1]["role"], "assistant");
        assert_eq!(
            normalized.messages[2]["content"],
            "List remaining regions.\nInclude ETA."
        );
        assert_eq!(
            normalized.system_prompt.as_deref(),
            Some("Answer as an operations analyst.")
        );
        assert_eq!(normalized.model.as_deref(), Some("gemini-2.0-flash"));
    }

    #[test]
    fn test_normalize_invalid_json_returns_none() {
        assert!(normalize_request(WebAppProvider::ClaudeWeb, br#"{"messages": [}"#).is_none());
    }

    #[test]
    fn test_normalize_claude_web_extracts_system_prompt() {
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "messages": [{"role": "user", "content": "Draft a migration checklist."}],
            "system": "Prioritize reversible database changes."
        });
        let normalized = normalize_json(WebAppProvider::ClaudeWeb, &body);

        assert_eq!(
            normalized.system_prompt.as_deref(),
            Some("Prioritize reversible database changes.")
        );
    }

    #[test]
    fn test_normalize_gemini_web_maps_model_to_assistant() {
        let body = json!({
            "contents": [{"role": "model", "parts": [{"text": "Build completed successfully."}]}],
            "generationConfig": {"model": "gemini-2.0-flash"}
        });
        let normalized = normalize_json(WebAppProvider::GeminiWeb, &body);

        assert_eq!(normalized.messages[0]["role"], "assistant");
    }

    #[test]
    fn test_estimate_tokens_uses_four_characters_per_token() {
        let messages = vec![json!({"role": "user", "content": "abcdefghijkl"})];
        assert_eq!(estimate_tokens(&messages), 3);
    }

    fn normalize_json(provider: WebAppProvider, body: &Value) -> super::NormalizedRequest {
        let bytes = serde_json::to_vec(body).expect("fixture should serialize");
        normalize_request(provider, &bytes).expect("fixture should normalize")
    }
}
