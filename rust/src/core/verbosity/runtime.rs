//! Runtime wiring for verbosity learning from tool dispatch receipts.

use std::sync::{LazyLock, Mutex};

use chrono::Utc;

use crate::core::cognitive_gate::full_science_enabled;

use super::{TranscriptEntry, extract_signals, recommend_level};

static TRANSCRIPT_BUFFER: LazyLock<Mutex<Vec<TranscriptEntry>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

const RECOMMEND_INTERVAL: usize = 10;
const MAX_TRANSCRIPT_ENTRIES: usize = 200;

/// Record a tool call in the session transcript buffer for verbosity learning.
pub(crate) fn record_tool_call(
    tool: &str,
    action: Option<&str>,
    args: &serde_json::Map<String, serde_json::Value>,
    output_tokens: usize,
) {
    if !full_science_enabled() {
        return;
    }

    let entry = TranscriptEntry {
        tool: tool.to_string(),
        target: transcript_target(args),
        compression_level: transcript_compression_level(args, action),
        response_tokens: output_tokens,
        timestamp: Utc::now(),
    };

    let Ok(mut buffer) = TRANSCRIPT_BUFFER.lock() else {
        return;
    };
    buffer.push(entry);
    if buffer.len() > MAX_TRANSCRIPT_ENTRIES {
        buffer.remove(0);
    }
    let len = buffer.len();
    if len.is_multiple_of(RECOMMEND_INTERVAL) {
        let signals = extract_signals(&buffer);
        let profile = recommend_level(&signals);
        tracing::debug!(
            "verbosity: recommended compression level: {}",
            compression_level_label(profile.level)
        );
    }
}

fn transcript_target(args: &serde_json::Map<String, serde_json::Value>) -> String {
    for key in ["path", "pattern", "query", "target", "symbol", "command"] {
        if let Some(value) = args.get(key).and_then(|v| v.as_str()) {
            return value.to_string();
        }
    }
    String::new()
}

fn transcript_compression_level(
    args: &serde_json::Map<String, serde_json::Value>,
    action: Option<&str>,
) -> String {
    args.get("mode")
        .and_then(|v| v.as_str())
        .or(action)
        .unwrap_or("standard")
        .to_string()
}

fn compression_level_label(level: crate::core::config::CompressionLevel) -> &'static str {
    use crate::core::config::CompressionLevel;
    match level {
        CompressionLevel::Off => "off",
        CompressionLevel::Lite => "lite",
        CompressionLevel::Standard => "standard",
        CompressionLevel::Max => "max",
        CompressionLevel::Raw => "raw",
    }
}
