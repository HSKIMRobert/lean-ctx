use std::collections::HashMap;

use serde_json::Value;

use super::tool_kind::{classify_tool_name, should_protect, ToolResultKind};

/// Summarize old tool_result blocks in conversation history to reduce token count.
/// Only prunes results older than `keep_recent` messages from the end.
///
/// `tool_names` maps the originating tool-call id → tool name so a pruned *file
/// read* is replaced with an honest, actionable stub ("re-read the file") rather
/// than a misleading first-3/last-2 excerpt of source code. Command/log output
/// keeps the head/tail summary, which stays readable for diagnostics.
pub fn prune_history(
    messages: &mut [Value],
    keep_recent: usize,
    tool_names: &HashMap<String, String>,
) {
    let len = messages.len();
    if len <= keep_recent {
        return;
    }
    let prune_end = len - keep_recent;

    for msg in &mut messages[..prune_end] {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");

        match role {
            // Anthropic: user messages with tool_result content blocks
            "user" => {
                if let Some(content) = msg.get_mut("content").and_then(|c| c.as_array_mut()) {
                    for block in content.iter_mut() {
                        if block.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                            let kind = block
                                .get("tool_use_id")
                                .and_then(|v| v.as_str())
                                .and_then(|id| tool_names.get(id))
                                .map_or(ToolResultKind::Other, |n| classify_tool_name(n));
                            summarize_anthropic_tool_result(block, kind);
                        }
                    }
                }
            }
            // OpenAI: tool role messages
            "tool" => {
                let kind = msg
                    .get("tool_call_id")
                    .and_then(|v| v.as_str())
                    .and_then(|id| tool_names.get(id))
                    .map_or(ToolResultKind::Other, |n| classify_tool_name(n));
                if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                    if content.len() > 200 {
                        let summary = summarize_or_stub(content, kind);
                        msg["content"] = Value::String(summary);
                    }
                }
            }
            _ => {}
        }
    }
}

fn summarize_anthropic_tool_result(block: &mut Value, kind: ToolResultKind) {
    if let Some(inner) = block.get_mut("content") {
        match inner {
            Value::String(s) if s.len() > 200 => {
                *s = summarize_or_stub(s, kind);
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                        if let Some(Value::String(s)) = item.get_mut("text") {
                            if s.len() > 200 {
                                *s = summarize_or_stub(s, kind);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// For a *protected* (file/source) result, emit an honest re-read stub. For
/// everything else, head/tail summarize so diagnostics stay readable.
fn summarize_or_stub(text: &str, kind: ToolResultKind) -> String {
    if should_protect(kind, text) {
        let lines = text.lines().count();
        return format!(
            "[lean-ctx: an earlier file read ({lines} lines) was pruned from older context to save tokens. Re-read the file if you need its full contents again.]"
        );
    }
    summarize_text(text)
}

fn summarize_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= 5 {
        return text.to_string();
    }

    let first_3: Vec<&str> = lines.iter().take(3).copied().collect();
    let last_2: Vec<&str> = lines.iter().rev().take(2).rev().copied().collect();

    format!(
        "{}\n[...{} lines pruned by lean-ctx...]\n{}",
        first_3.join("\n"),
        lines.len() - 5,
        last_2.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_names() -> HashMap<String, String> {
        HashMap::new()
    }

    #[test]
    fn prune_skips_recent_messages() {
        let long_content = (0..40).map(|i| format!("line {i}: this is a longer line to ensure content exceeds the 200 character threshold for pruning")).collect::<Vec<_>>().join("\n");
        let mut messages = vec![
            serde_json::json!({"role": "tool", "content": long_content}),
            serde_json::json!({"role": "assistant", "content": "ok"}),
            serde_json::json!({"role": "tool", "content": long_content}),
        ];
        prune_history(&mut messages, 2, &no_names());
        let first = messages[0]["content"].as_str().unwrap();
        assert!(first.contains("pruned"), "old message should be pruned");
        let last = messages[2]["content"].as_str().unwrap();
        assert!(!last.contains("pruned"), "recent message should be kept");
    }

    #[test]
    fn prune_handles_short_content() {
        let mut messages = vec![serde_json::json!({"role": "tool", "content": "short"})];
        prune_history(&mut messages, 0, &no_names());
        assert_eq!(messages[0]["content"].as_str().unwrap(), "short");
    }

    #[test]
    fn old_file_read_gets_honest_reread_stub() {
        let code = (0..40)
            .map(|i| format!("    let value_{i} = compute_{i}(input);"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut names = HashMap::new();
        names.insert("call_1".to_string(), "read_file".to_string());
        let mut messages = vec![
            serde_json::json!({"role": "tool", "tool_call_id": "call_1", "content": code}),
            serde_json::json!({"role": "assistant", "content": "ok"}),
            serde_json::json!({"role": "user", "content": "next"}),
        ];
        prune_history(&mut messages, 2, &names);
        let stub = messages[0]["content"].as_str().unwrap();
        assert!(
            stub.contains("Re-read the file"),
            "code read should get re-read stub, got: {stub}"
        );
        assert!(
            !stub.contains("value_5"),
            "source body must not be partially leaked"
        );
    }

    #[test]
    fn old_log_output_keeps_head_tail_summary() {
        let log = (0..40)
            .map(|i| format!("INFO line {i}: processing item number {i} in the batch run"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut names = HashMap::new();
        names.insert("call_1".to_string(), "Bash".to_string());
        let mut messages = vec![
            serde_json::json!({"role": "tool", "tool_call_id": "call_1", "content": log}),
            serde_json::json!({"role": "assistant", "content": "ok"}),
            serde_json::json!({"role": "user", "content": "next"}),
        ];
        prune_history(&mut messages, 2, &names);
        let summary = messages[0]["content"].as_str().unwrap();
        assert!(
            summary.contains("lines pruned by lean-ctx"),
            "logs keep head/tail summary"
        );
    }
}
