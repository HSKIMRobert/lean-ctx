//! Cache-aligner volatile-field detection (#940, Headroom "cache aligner" stage
//! 1) — **telemetry-first**.
//!
//! A stable system prompt is the largest prefix a provider can cache, but a
//! single turn-to-turn-varying token inside it (today's date, a fresh UUID, a
//! git SHA) shifts the bytes and busts the cache on every request. Headroom's
//! cache aligner *relocates* those volatile fields to the tail so the prefix
//! stays byte-stable. Relocating provider-visible system content is risky, so
//! this phase ships only the **measurement** half: a deterministic detector that
//! counts the volatile fields in an unanchored system prompt, surfaced on
//! `/status` so a user can see how much cache their prompt is leaking before any
//! opt-in relocate is enabled.
//!
//! ## Why measure first
//! The honest, low-risk order is: detect → quantify (telemetry) → only then offer
//! an opt-in tail-relocate behind its own flag, once the data shows it pays. The
//! relocate, when added, will reuse the stable-first ordering of
//! [`crate::core::neural::cache_alignment::CacheAlignedOutput`] (today only
//! exercised by the doctor self-test) as its building block.
//!
//! ## Determinism (#498)
//! The scan is a pure function of the text: every pattern's matches are collected,
//! sorted, and overlapping spans merged, so the field count and covered-byte total
//! are stable across runs and never depend on hash-map order. It mutates nothing —
//! the request body is byte-identical whether the scan runs or not.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

/// Volatile substrings that change turn-to-turn and so bust an otherwise-stable
/// system-prompt prefix. Deliberately precise (ISO dates/datetimes, UUIDs, full
/// git SHAs) rather than broad, so a stable identifier is never miscounted as
/// volatile. Datetimes are matched alongside bare dates; the span merge below
/// collapses the overlap so a full timestamp counts once.
static VOLATILE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        // ISO-8601 datetime: date + time, optional seconds/fraction/zone.
        r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}(?::\d{2})?(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?",
        // ISO-8601 date.
        r"\d{4}-\d{2}-\d{2}",
        // RFC-4122 UUID.
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
        // git SHA-1 (40 lowercase hex), a common volatile "current commit" field.
        r"\b[0-9a-f]{40}\b",
    ]
    .iter()
    .filter_map(|p| Regex::new(p).ok())
    .collect()
});

/// Result of scanning a system prompt for volatile, cache-busting fields.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct VolatileScan {
    /// Number of distinct (overlap-merged) volatile spans found.
    pub fields: usize,
    /// Total bytes covered by those spans — how much of the prefix is volatile.
    pub volatile_bytes: usize,
}

/// Deterministically scan `text` for volatile fields, merging overlapping matches
/// (e.g. a datetime and the bare date inside it) so each is counted once.
pub(crate) fn scan_volatile(text: &str) -> VolatileScan {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for re in VOLATILE_PATTERNS.iter() {
        spans.extend(re.find_iter(text).map(|m| (m.start(), m.end())));
    }
    if spans.is_empty() {
        return VolatileScan::default();
    }
    spans.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(spans.len());
    for (start, end) in spans {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    VolatileScan {
        fields: merged.len(),
        volatile_bytes: merged.iter().map(|(s, e)| e - s).sum(),
    }
}

/// The plain text of an Anthropic `system` field — a bare string, or every text
/// block of a block array joined with newlines. `None` for any other shape.
pub(crate) fn system_text(system: &Value) -> Option<String> {
    match system {
        Value::String(s) => Some(s.clone()),
        Value::Array(blocks) => {
            let joined = blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n");
            (!joined.is_empty()).then_some(joined)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_each_volatile_kind_once() {
        let text = "Today is 2026-06-22. Session 550e8400-e29b-41d4-a716-446655440000 \
                    at commit da39a3ee5e6b4b0d3255bfef95601890afd80709.";
        let scan = scan_volatile(text);
        assert_eq!(scan.fields, 3, "one date, one UUID, one git SHA");
        assert!(scan.volatile_bytes > 0);
    }

    #[test]
    fn datetime_and_inner_date_merge_to_one_span() {
        // The datetime pattern and the bare-date pattern both match the date part;
        // the merge must collapse them so a full timestamp counts exactly once.
        let scan = scan_volatile("Generated at 2026-06-22T15:04:05Z by the agent.");
        assert_eq!(
            scan.fields, 1,
            "overlapping datetime/date spans merge to one"
        );
    }

    #[test]
    fn stable_prompt_has_no_volatile_fields() {
        let scan = scan_volatile("You are a careful senior engineer. Prefer small diffs.");
        assert_eq!(scan, VolatileScan::default());
    }

    #[test]
    fn scan_is_deterministic() {
        let text = "v1 2026-06-22 id 550e8400-e29b-41d4-a716-446655440000 and 2025-01-01";
        assert_eq!(scan_volatile(text), scan_volatile(text));
    }

    #[test]
    fn system_text_reads_string_and_block_array() {
        assert_eq!(
            system_text(&Value::String("hi".into())).as_deref(),
            Some("hi")
        );
        let arr = serde_json::json!([
            {"type": "text", "text": "alpha"},
            {"type": "text", "text": "beta"}
        ]);
        assert_eq!(system_text(&arr).as_deref(), Some("alpha\nbeta"));
        assert_eq!(system_text(&serde_json::json!(42)), None);
    }
}
