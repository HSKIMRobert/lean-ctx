use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::Response,
};
use serde_json::Value;

use super::compress::compress_tool_result;
use super::forward;
use super::tool_kind::{self, should_protect, ToolResultKind};
use super::ProxyState;

const KEEP_RECENT: usize = 6;

pub async fn handler(
    State(state): State<ProxyState>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
    let upstream = state.openai_upstream.clone();
    forward::forward_request(
        State(state),
        req,
        &upstream,
        "/v1/chat/completions",
        compress_request_body,
        "OpenAI",
        &[],
    )
    .await
}

fn compress_request_body(body: &[u8]) -> (Vec<u8>, usize, usize) {
    let original_size = body.len();

    let parsed: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return (body.to_vec(), original_size, original_size),
    };

    let mut doc = parsed;
    let mut modified = false;

    if let Some(messages) = doc.get_mut("messages").and_then(|m| m.as_array_mut()) {
        let tool_names = tool_kind::openai_tool_names(messages);

        super::history_prune::prune_history(messages, KEEP_RECENT, &tool_names);
        modified = true;

        for msg in messages.iter_mut() {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
            if role != "tool" {
                continue;
            }

            let name = msg
                .get("tool_call_id")
                .and_then(|v| v.as_str())
                .and_then(|id| tool_names.get(id))
                .map(String::as_str);
            let kind = name.map_or(ToolResultKind::Other, tool_kind::classify_tool_name);

            if let Some(content) = msg
                .get_mut("content")
                .and_then(|c| c.as_str().map(String::from))
            {
                if should_protect(kind, &content) {
                    continue;
                }
                let compressed = compress_tool_result(&content, name);
                if compressed.len() < content.len() {
                    msg["content"] = Value::String(compressed);
                    modified = true;
                }
            }
        }
    }

    if !modified {
        return (body.to_vec(), original_size, original_size);
    }

    match serde_json::to_vec(&doc) {
        Ok(compressed) => {
            let compressed_size = compressed.len();
            (compressed, original_size, compressed_size)
        }
        Err(_) => (body.to_vec(), original_size, original_size),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_file_tool_result_protected() {
        let code = (0..60)
            .map(|i| format!("    const value{i} = computeValue{i}(ctx, opts);"))
            .collect::<Vec<_>>()
            .join("\n");
        let body = serde_json::json!({
            "model": "gpt-5",
            "messages": [
                {"role": "assistant", "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "read_file"}}]},
                {"role": "tool", "tool_call_id": "call_1", "content": code}
            ]
        });
        let bytes = serde_json::to_vec(&body).unwrap();
        let (out, _orig, _comp) = compress_request_body(&bytes);
        let parsed: Value = serde_json::from_slice(&out).unwrap();
        assert!(parsed["messages"][1]["content"]
            .as_str()
            .unwrap()
            .contains("value59"));
    }
}
