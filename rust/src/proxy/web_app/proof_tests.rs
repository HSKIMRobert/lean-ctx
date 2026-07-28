use super::conversation_tracker::ConversationTracker;
use super::normalize::normalize_request;
use super::{NormalizedRequest, WebAppProvider, detect_web_provider};
use serde_json::json;

#[test]
fn proof_claude_web_25_turn_compression() {
    let mut messages = vec![];
    for i in 0..25 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        let content = format!(
            "Turn {} content with enough text to simulate real conversation. {}",
            i,
            "This is additional context to make the message realistic and long enough for compression to be meaningful. ".repeat(3)
        );
        messages.push(json!({"role": role, "content": content}));
    }

    let body = json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 4096,
        "messages": messages,
        "system": "You are a helpful coding assistant with deep knowledge of Rust."
    });

    let body_bytes = serde_json::to_vec(&body).unwrap();
    let normalized = normalize_request(WebAppProvider::ClaudeWeb, &body_bytes);
    assert!(normalized.is_some(), "Claude.ai request must normalize");
    let normalized: NormalizedRequest = normalized.unwrap();
    assert_eq!(normalized.provider, WebAppProvider::ClaudeWeb);
    assert_eq!(normalized.messages.len(), 25);
    assert!(normalized.system_prompt.is_some());
    assert_eq!(
        normalized.model.as_deref(),
        Some("claude-sonnet-4-20250514")
    );
    assert!(
        normalized.messages.len() > 12,
        "enough turns for compression"
    );
}

#[test]
fn proof_chatgpt_state_tracking_full_cycle() {
    let mut tracker = ConversationTracker::new(10, 100);
    let conv_id = "86f73b54-0f51-47ba-84a3-07c1e25dce81";
    let load_response = json!({
        "mapping": {
            "root": {
                "id": "root",
                "parent": null,
                "message": {
                    "id": "root",
                    "author": {"role": "system"},
                    "content": {
                        "content_type": "text",
                        "parts": ["You are helpful"]
                    }
                }
            },
            "msg-1": {
                "id": "msg-1",
                "parent": "root",
                "message": {
                    "id": "msg-1",
                    "author": {"role": "user"},
                    "content": {"content_type": "text", "parts": ["Hello!"]}
                }
            },
            "msg-2": {
                "id": "msg-2",
                "parent": "msg-1",
                "message": {
                    "id": "msg-2",
                    "author": {"role": "assistant"},
                    "content": {
                        "content_type": "text",
                        "parts": ["Hi there! How can I help?"]
                    }
                }
            }
        },
        "current_node": "msg-2",
        "title": "Test Chat"
    });
    let load_bytes = serde_json::to_vec(&load_response).unwrap();
    tracker.track_conversation_load(conv_id, &load_bytes);
    assert!(tracker.has_conversation(conv_id));

    tracker.track_sse_response(conv_id, "msg-4", "assistant", "The answer is 42.");

    let new_turn = json!({
        "role": "user",
        "content": "What is the meaning of life?"
    });
    let history = tracker.reconstruct_with_new_turn(conv_id, &new_turn);
    assert!(history.is_some(), "tracked conversation must reconstruct");
    let history = history.unwrap();
    assert!(history.len() >= 4, "must have tracked history + new turn");

    let last = history.last().unwrap();
    assert_eq!(last["role"], "user");
    assert_eq!(last["content"], "What is the meaning of life?");
}

#[test]
fn proof_gemini_web_parts_normalization() {
    let body = json!({
        "contents": [
            {"role": "user", "parts": [{"text": "Explain quantum computing"}]},
            {"role": "model", "parts": [{"text": "Quantum computing uses qubits..."}]},
            {"role": "user", "parts": [{"text": "How does entanglement work?"}]}
        ],
        "systemInstruction": {"parts": [{"text": "You are a physics professor"}]},
        "generationConfig": {"model": "gemini-2.0-flash"}
    });

    let body_bytes = serde_json::to_vec(&body).unwrap();
    let normalized = normalize_request(WebAppProvider::GeminiWeb, &body_bytes);
    assert!(normalized.is_some(), "Gemini request must normalize");

    let normalized = normalized.unwrap();
    assert_eq!(normalized.provider, WebAppProvider::GeminiWeb);
    assert_eq!(normalized.messages.len(), 3);
    assert_eq!(normalized.messages[1]["role"], "assistant");
    assert_eq!(
        normalized.messages[1]["content"],
        "Quantum computing uses qubits..."
    );
    assert_eq!(
        normalized.system_prompt.as_deref(),
        Some("You are a physics professor")
    );
}

#[test]
fn proof_web_provider_detection_all_providers() {
    assert_eq!(
        detect_web_provider(
            "claude.ai",
            "/api/organizations/org/chat_conversations/123/completion"
        ),
        Some(WebAppProvider::ClaudeWeb)
    );
    assert_eq!(
        detect_web_provider("chatgpt.com", "/backend-api/conversation"),
        Some(WebAppProvider::ChatGptWeb)
    );
    assert_eq!(
        detect_web_provider("chat.openai.com", "/backend-api/conversation"),
        Some(WebAppProvider::ChatGptWeb)
    );
    assert_eq!(
        detect_web_provider("gemini.google.com", "/v1beta/models/gemini/generateContent"),
        Some(WebAppProvider::GeminiWeb)
    );
    assert_eq!(detect_web_provider("example.com", "/api/chat"), None);
    assert_eq!(
        detect_web_provider("api.anthropic.com", "/v1/messages"),
        None,
        "API endpoints are handled by existing proxy, not web_app module"
    );
}

#[test]
fn proof_normalize_rejects_invalid_input() {
    assert!(normalize_request(WebAppProvider::ClaudeWeb, b"not json").is_none());
    assert!(normalize_request(WebAppProvider::ClaudeWeb, b"{}").is_none());
    assert!(normalize_request(WebAppProvider::GeminiWeb, b"[]").is_none());
    assert!(normalize_request(WebAppProvider::ChatGptWeb, b"{\"action\": \"next\"}").is_none());
}

#[test]
fn proof_chatgpt_tracker_lru_eviction() {
    let mut tracker = ConversationTracker::new(2, 50);

    let minimal_conv = |id: &str| {
        json!({
            "mapping": {
                "root": {
                    "id": "root",
                    "parent": null,
                    "message": {
                        "id": "root",
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": [id]}
                    }
                }
            },
            "current_node": "root"
        })
    };

    tracker.track_conversation_load(
        "conv-1",
        &serde_json::to_vec(&minimal_conv("conv-1")).unwrap(),
    );
    tracker.track_conversation_load(
        "conv-2",
        &serde_json::to_vec(&minimal_conv("conv-2")).unwrap(),
    );
    assert_eq!(tracker.conversation_count(), 2);

    tracker.track_conversation_load(
        "conv-3",
        &serde_json::to_vec(&minimal_conv("conv-3")).unwrap(),
    );
    assert_eq!(tracker.conversation_count(), 2);
    assert!(
        !tracker.has_conversation("conv-1"),
        "oldest must be evicted"
    );
    assert!(tracker.has_conversation("conv-2"));
    assert!(tracker.has_conversation("conv-3"));
}

#[test]
fn proof_token_estimation_reasonable() {
    use super::normalize::estimate_tokens;

    let messages = vec![
        json!({"role": "user", "content": "Hello world"}),
        json!({
            "role": "assistant",
            "content": "Hi there, how can I help you today?"
        }),
    ];
    let tokens = estimate_tokens(&messages);
    assert!(tokens > 5, "should estimate at least some tokens");
    assert!(tokens < 50, "should not wildly overestimate");
}

#[test]
fn proof_tracker_unknown_conversation_returns_none() {
    let tracker = ConversationTracker::new(10, 100);
    let new_turn = json!({"role": "user", "content": "Hello"});
    assert!(
        tracker
            .reconstruct_with_new_turn("nonexistent", &new_turn)
            .is_none()
    );
}
