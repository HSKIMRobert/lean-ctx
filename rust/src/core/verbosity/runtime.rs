//! Runtime wiring for verbosity learning from tool dispatch receipts.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

use chrono::Utc;

use crate::core::cognitive_gate::full_science_enabled;
use crate::core::config::{CompressionLevel, Config};

use super::{TranscriptEntry, extract_signals, recommend_level};

static TRANSCRIPT_BUFFER: LazyLock<Mutex<Vec<TranscriptEntry>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

static RECOMMENDED_LEVEL: LazyLock<Mutex<Option<CompressionLevel>>> =
    LazyLock::new(|| Mutex::new(None));

static AUTO_APPLIED: AtomicBool = AtomicBool::new(false);

const RECOMMEND_INTERVAL: usize = 10;
const MAX_TRANSCRIPT_ENTRIES: usize = 200;

/// Whether verbosity auto-apply stored a more aggressive level this session.
pub(crate) fn auto_apply_happened() -> bool {
    AUTO_APPLIED.load(Ordering::Relaxed)
}

/// Latest verbosity-learned compression level awaiting application.
pub(crate) fn recommended_compression() -> Option<CompressionLevel> {
    *RECOMMENDED_LEVEL
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

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
        maybe_auto_apply_recommendation(profile.level);
    }
}

fn maybe_auto_apply_recommendation(recommended: CompressionLevel) {
    // Correction-loop degrade lowers compression — never override it with a boost.
    if CompressionLevel::session_degrade_level().is_some() {
        return;
    }

    let cfg = Config::load();
    let current = CompressionLevel::effective(&cfg);
    if !recommended.is_more_aggressive_than(&current) {
        return;
    }

    let Ok(mut slot) = RECOMMENDED_LEVEL.lock() else {
        return;
    };
    let should_store = slot
        .as_ref()
        .is_none_or(|existing| recommended.is_more_aggressive_than(existing));
    if !should_store {
        return;
    }

    tracing::info!(
        "[verbosity] auto-applying compression level: {}",
        compression_level_label(recommended)
    );
    slot.replace(recommended);
    AUTO_APPLIED.store(true, Ordering::Relaxed);
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

fn compression_level_label(level: CompressionLevel) -> &'static str {
    level.label()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::CompressionLevel;
    use crate::core::verbosity::BehaviorSignal;
    use serial_test::serial;

    #[test]
    #[serial]
    fn verbosity_auto_apply_stores_more_aggressive_level() {
        CompressionLevel::clear_session_degrade();
        let Ok(mut slot) = RECOMMENDED_LEVEL.lock() else {
            return;
        };
        *slot = None;
        drop(slot);

        let signals = vec![BehaviorSignal::TaskComplete { reads_count: 2 }];
        maybe_auto_apply_recommendation(recommend_level(&signals).level);

        let recommended = recommended_compression();
        assert!(
            recommended.is_some_and(|level| level.is_more_aggressive_than(&CompressionLevel::Lite)),
            "expected a more-aggressive recommendation, got {recommended:?}"
        );
    }

    #[test]
    #[serial]
    fn verbosity_never_recommends_less_compression() {
        CompressionLevel::clear_session_degrade();
        let Ok(mut slot) = RECOMMENDED_LEVEL.lock() else {
            return;
        };
        *slot = Some(CompressionLevel::Max);
        drop(slot);

        maybe_auto_apply_recommendation(CompressionLevel::Lite);
        assert_eq!(recommended_compression(), Some(CompressionLevel::Max));
    }
}
