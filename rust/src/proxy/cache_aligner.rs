//! Cache-aligner (#940 detect, #974 relocate) — Headroom "cache aligner" port.
//!
//! A stable system prompt is the largest prefix a provider can cache, but a
//! single turn-to-turn-varying token inside it (today's date, a fresh UUID, a
//! git SHA) shifts the bytes and busts the cache on every request. Two opt-in
//! stages address this, both Anthropic-only:
//!
//! 1. **Detect** (`cache_aligner`, #940): a deterministic scan counts the
//!    volatile fields in an *unanchored* system prompt and surfaces the leak on
//!    `/status` — pure measurement, the body is never mutated.
//! 2. **Relocate** (`cache_align_relocate`, #974): rewrites `system` into a
//!    stable block (volatile values replaced by constant placeholders) carrying
//!    the cache breakpoint, plus an *uncached* tail block that re-states the
//!    relocated values. The cacheable prefix then stays byte-stable turn-to-turn
//!    and finally caches; only the small, reprocessed tail changes. Follows the
//!    same stable-first ordering as
//!    [`crate::core::neural::cache_alignment::CacheAlignedOutput`].
//!
//! ## Determinism (#498) & cache-safety (#448)
//! Both stages are pure functions of the text: matches come from [`merged_spans`]
//! (collected, sorted, overlaps merged), and the relocate's placeholders + tail
//! header are byte-constants, so identical input yields byte-identical output and
//! the rewritten prefix is stable across turns. The relocate is idempotent (a
//! second pass sees only placeholders) and only ever fires when the client
//! anchored nothing itself, so it never rewrites a client-cached prefix.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value};

use crate::core::tokens::count_tokens;

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

/// Deterministically collect the volatile spans in `text`, merging overlapping
/// matches (e.g. a datetime and the bare date inside it) so each counts once.
/// Shared by the detector ([`scan_volatile`]) and the relocate
/// ([`relocate_volatile`]) so both see exactly the same fields.
fn merged_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for re in VOLATILE_PATTERNS.iter() {
        spans.extend(re.find_iter(text).map(|m| (m.start(), m.end())));
    }
    if spans.is_empty() {
        return spans;
    }
    spans.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(spans.len());
    for (start, end) in spans {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// Deterministically scan `text` for volatile fields (measurement-only, #940).
pub(crate) fn scan_volatile(text: &str) -> VolatileScan {
    let merged = merged_spans(text);
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

/// Anthropic ignores a cache breakpoint whose prefix is under its minimum
/// cacheable size; relocating below it just churns bytes for no cache win, so the
/// relocate is gated on the same floor as `cache_breakpoint` (#939).
const MIN_STABLE_TOKENS: usize = 1024;

/// Constant header introducing the relocated tail block. Byte-constant so it
/// never perturbs the prefix (#498).
const TAIL_HEADER: &str = "Volatile context (relocated to keep the prompt-cache prefix stable):";

/// The constant placeholder that replaces the `n`-th relocated value in the
/// stable block. Numbered by appearance so the model can map it to the tail and
/// so the rewrite is deterministic; carries no volatile pattern, which is what
/// makes [`relocate_volatile`] idempotent.
fn placeholder(n: usize) -> String {
    format!("[ctx#{n}]")
}

/// A system prompt split for cache alignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelocateResult {
    /// System text with every volatile value replaced by a constant placeholder
    /// — byte-stable turn-to-turn, so it is the part that caches.
    pub stable: String,
    /// The relocated volatile values, re-stated in order under a constant header.
    /// Belongs in an *uncached* trailing block.
    pub tail: String,
    /// Number of volatile fields relocated.
    pub fields: usize,
}

/// Split `text` into a byte-stable `stable` part (volatile values → placeholders)
/// and a `tail` that re-states those values. `None` when there is nothing
/// volatile to move, so callers stay a strict no-op.
pub(crate) fn relocate_volatile(text: &str) -> Option<RelocateResult> {
    let spans = merged_spans(text);
    if spans.is_empty() {
        return None;
    }
    let mut stable = String::with_capacity(text.len());
    let mut values: Vec<&str> = Vec::with_capacity(spans.len());
    let mut cursor = 0usize;
    for (start, end) in &spans {
        stable.push_str(&text[cursor..*start]);
        stable.push_str(&placeholder(values.len() + 1));
        values.push(&text[*start..*end]);
        cursor = *end;
    }
    stable.push_str(&text[cursor..]);

    let mut tail = String::from(TAIL_HEADER);
    for (i, value) in values.iter().enumerate() {
        tail.push('\n');
        tail.push_str(&placeholder(i + 1));
        tail.push_str(" = ");
        tail.push_str(value);
    }
    Some(RelocateResult {
        stable,
        tail,
        fields: values.len(),
    })
}

/// A plain `{"type":"text","text":…}` system block.
fn text_block(text: String) -> Map<String, Value> {
    let mut block = Map::new();
    block.insert("type".into(), Value::String("text".into()));
    block.insert("text".into(), Value::String(text));
    block
}

/// The stable block plus the ephemeral cache breakpoint that anchors the prefix.
fn stable_block(text: String) -> Value {
    let mut block = text_block(text);
    block.insert(
        "cache_control".into(),
        serde_json::json!({ "type": "ephemeral" }),
    );
    Value::Object(block)
}

/// Rewrite the Anthropic `system` field in place so volatile values live in an
/// uncached tail block and the stable prefix carries the cache breakpoint.
/// Returns the number of fields relocated (`0` = left untouched).
///
/// Handles a plain string or an array of pure text blocks that carry no
/// `cache_control` of their own (the caller already guards anchored prefixes).
/// Any other shape, or a stable part below [`MIN_STABLE_TOKENS`], is a no-op.
pub(crate) fn apply_anthropic_relocate(doc: &mut Value) -> usize {
    let Some(system) = doc.get_mut("system") else {
        return 0;
    };
    let text = match system {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => {
            let all_plain_text = !blocks.is_empty()
                && blocks.iter().all(|b| {
                    b.get("type").and_then(Value::as_str) == Some("text")
                        && b.get("text").is_some_and(Value::is_string)
                        && b.get("cache_control").is_none()
                });
            if !all_plain_text {
                return 0;
            }
            blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => return 0,
    };
    let Some(result) = relocate_volatile(&text) else {
        return 0;
    };
    if count_tokens(&result.stable) < MIN_STABLE_TOKENS {
        return 0;
    }
    *system = Value::Array(vec![
        stable_block(result.stable),
        Value::Object(text_block(result.tail)),
    ]);
    result.fields
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

    // A system prompt comfortably over MIN_STABLE_TOKENS, with one volatile date.
    fn big_system_with_date() -> String {
        format!(
            "You are a meticulous senior engineer. Today is 2026-06-27. {}",
            "Prefer small, well-tested diffs. ".repeat(400)
        )
    }

    #[test]
    fn relocate_moves_volatiles_to_tail_and_leaves_placeholders() {
        let result =
            relocate_volatile("Date 2026-06-27, id 550e8400-e29b-41d4-a716-446655440000.").unwrap();
        assert_eq!(result.fields, 2);
        assert!(!result.stable.contains("2026-06-27"), "value left prefix");
        assert!(result.stable.contains("[ctx#1]") && result.stable.contains("[ctx#2]"));
        assert!(result.tail.contains("[ctx#1] = 2026-06-27"));
        assert!(
            result
                .tail
                .contains("[ctx#2] = 550e8400-e29b-41d4-a716-446655440000")
        );
    }

    #[test]
    fn relocate_is_noop_without_volatile_fields() {
        assert!(relocate_volatile("You are a careful engineer.").is_none());
    }

    #[test]
    fn relocate_is_idempotent() {
        let once = relocate_volatile("Built at 2026-06-27 ok").unwrap();
        assert!(
            relocate_volatile(&once.stable).is_none(),
            "placeholders carry no volatile pattern, so a second pass is a no-op"
        );
    }

    #[test]
    fn relocate_is_deterministic() {
        let text = "v 2026-06-27 id 550e8400-e29b-41d4-a716-446655440000 sha \
                    da39a3ee5e6b4b0d3255bfef95601890afd80709";
        assert_eq!(relocate_volatile(text), relocate_volatile(text));
    }

    #[test]
    fn apply_rewrites_string_system_into_stable_plus_tail() {
        let mut doc = serde_json::json!({ "system": big_system_with_date(), "messages": [] });
        assert_eq!(apply_anthropic_relocate(&mut doc), 1);
        let system = &doc["system"];
        assert!(system.is_array(), "string system becomes a block array");
        assert_eq!(system[0]["cache_control"]["type"], "ephemeral");
        assert!(
            !system[0]["text"].as_str().unwrap().contains("2026-06-27"),
            "the date left the cacheable prefix"
        );
        assert!(
            system[1].get("cache_control").is_none(),
            "the tail block stays uncached"
        );
        assert!(
            system[1]["text"].as_str().unwrap().contains("2026-06-27"),
            "the date was relocated to the tail"
        );
    }

    #[test]
    fn apply_skips_small_system_and_clean_system() {
        let mut small = serde_json::json!({ "system": "Today is 2026-06-27", "messages": [] });
        assert_eq!(
            apply_anthropic_relocate(&mut small),
            0,
            "below the cacheable floor → no churn"
        );
        let mut clean =
            serde_json::json!({ "system": "You are precise. ".repeat(400), "messages": [] });
        assert_eq!(
            apply_anthropic_relocate(&mut clean),
            0,
            "no volatile fields → strict no-op"
        );
    }

    #[test]
    fn apply_skips_array_with_existing_breakpoint() {
        let mut doc = serde_json::json!({
            "system": [{
                "type": "text",
                "text": big_system_with_date(),
                "cache_control": { "type": "ephemeral" }
            }],
            "messages": []
        });
        assert_eq!(
            apply_anthropic_relocate(&mut doc),
            0,
            "a client-anchored array must be left untouched"
        );
    }

    #[test]
    fn apply_is_deterministic() {
        let mk = || serde_json::json!({ "system": big_system_with_date(), "messages": [] });
        let (mut a, mut b) = (mk(), mk());
        assert_eq!(apply_anthropic_relocate(&mut a), 1);
        assert_eq!(apply_anthropic_relocate(&mut b), 1);
        assert_eq!(a, b, "identical input → byte-identical output (#498)");
    }
}
