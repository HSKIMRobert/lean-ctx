//! Record boundary detection and per-segment compression for multi-record output.
//!
//! Prevents the flat-stream compression pipeline from dropping delimiter lines
//! (like `===== #N =====`) and reattributing one record's body to another.
//!
//! See #1387.

use crate::core::tokens::{count_tokens_for, TokenizerFamily};

/// Minimum number of matching boundaries to activate per-segment compression.
const MIN_BOUNDARIES: usize = 3;

/// A detected record boundary in the output.
#[derive(Debug, Clone)]
pub(super) struct BoundarySpan {
    pub start: usize,
    pub end: usize,
    pub line: String,
}

/// Detects repeating delimiter lines that partition multi-record output.
///
/// Recognised patterns:
/// - `===== #N` / `===== Record N` / `===== Item N`
/// - `---- #N` / `---- Record N`
/// - `>>> #N`
/// - `========` (8+ repeated chars)
///
/// Returns `None` when fewer than `MIN_BOUNDARIES` matching lines are found.
pub(super) fn detect_record_boundaries(output: &str) -> Option<Vec<BoundarySpan>> {
    let mut boundaries = Vec::new();
    let mut shape: Option<DelimiterShape> = None;
    let mut byte_offset: usize = 0;

    for line in output.split('\n') {
        let line_bytes = line.len() + 1; // +1 for the '\n'
        let trimmed = line.trim();

        if let Some(detected) = classify_boundary_line(trimmed) {
            let is_match = match &shape {
                None => {
                    shape = Some(detected);
                    true
                }
                Some(existing) => existing.matches(&detected),
            };
            if is_match {
                boundaries.push(BoundarySpan {
                    start: byte_offset,
                    end: (byte_offset + line_bytes).min(output.len()),
                    line: line.to_string(),
                });
            }
        }

        byte_offset += line_bytes;
    }

    if boundaries.len() >= MIN_BOUNDARIES {
        Some(boundaries)
    } else {
        None
    }
}

/// Compresses each segment between boundaries independently, preserving all
/// delimiter lines verbatim in the output.
///
/// Returns `None` if per-segment compression yields no net savings (>10%)
/// over the original token count — callers then fall through to the normal
/// flat pipeline.
pub(super) fn compress_preserving_boundaries(
    command: &str,
    output: &str,
    exit_code: i32,
    family: TokenizerFamily,
    boundaries: &[BoundarySpan],
    original_tokens: usize,
) -> Option<String> {
    let segments = build_segments(output, boundaries);

    let mut result = String::with_capacity(output.len());
    let mut total_compressed_tokens: usize = 0;

    for segment in &segments {
        match segment {
            Segment::Boundary(line) => {
                result.push_str(line);
                result.push('\n');
                total_compressed_tokens += count_tokens_for(line, family);
            }
            Segment::Content(content) => {
                if content.trim().is_empty() {
                    if !content.is_empty() {
                        result.push('\n');
                    }
                    continue;
                }
                let compressed = compress_single_segment(command, content, exit_code, family);
                total_compressed_tokens += count_tokens_for(&compressed, family);
                result.push_str(&compressed);
                if !compressed.ends_with('\n') {
                    result.push('\n');
                }
            }
        }
    }

    let savings_threshold = original_tokens * 9 / 10;
    if total_compressed_tokens < savings_threshold {
        Some(result)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Per-segment compression (lightweight, no boundary re-detection)
// ---------------------------------------------------------------------------

/// Compresses a single record segment. Uses the terse pipeline + lightweight
/// cleanup but skips boundary detection (preventing infinite recursion) and
/// the 200-token floor (individual segments are typically small).
fn compress_single_segment(
    command: &str,
    content: &str,
    _exit_code: i32,
    family: TokenizerFamily,
) -> String {
    let original_tokens = count_tokens_for(content, family);

    // Very small segments: keep verbatim (not worth compressing).
    if original_tokens < 30 {
        return content.to_string();
    }

    // Try terse pipeline first.
    let cfg = crate::core::config::Config::load();
    let level = crate::core::config::CompressionLevel::effective(&cfg);
    if level.is_active() {
        let terse = crate::core::terse::pipeline::compress(content, &level, None);
        if terse.quality_passed && terse.savings_pct >= 5.0 {
            return terse.output;
        }
    }

    // Lightweight cleanup as fallback.
    let cleaned = crate::core::compressor::lightweight_cleanup(content);
    let cleaned_tokens = count_tokens_for(&cleaned, family);
    if cleaned_tokens < original_tokens {
        return cleaned;
    }

    content.to_string()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

enum Segment {
    Boundary(String),
    Content(String),
}

fn build_segments(output: &str, boundaries: &[BoundarySpan]) -> Vec<Segment> {
    let mut segments = Vec::with_capacity(boundaries.len() * 2 + 1);
    let mut cursor = 0;

    for b in boundaries {
        if b.start > cursor {
            segments.push(Segment::Content(output[cursor..b.start].to_string()));
        }
        segments.push(Segment::Boundary(b.line.clone()));
        cursor = b.end.min(output.len());
    }

    if cursor < output.len() {
        segments.push(Segment::Content(output[cursor..].to_string()));
    }

    segments
}

// ---------------------------------------------------------------------------
// Delimiter classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct DelimiterShape {
    prefix_char: char,
    #[allow(dead_code)]
    has_index: bool,
    is_pure_separator: bool,
}

impl DelimiterShape {
    fn matches(&self, other: &Self) -> bool {
        self.prefix_char == other.prefix_char && self.is_pure_separator == other.is_pure_separator
    }
}

fn classify_boundary_line(trimmed: &str) -> Option<DelimiterShape> {
    if trimmed.is_empty() {
        return None;
    }

    let first = trimmed.chars().next()?;

    // Pure separator: 8+ repeated chars
    if matches!(first, '=' | '-' | '*' | '#' | '~')
        && trimmed.len() >= 8
        && trimmed.chars().all(|c| c == first)
    {
        return Some(DelimiterShape {
            prefix_char: first,
            has_index: false,
            is_pure_separator: true,
        });
    }

    let prefixes: &[(&str, char)] = &[("=====", '='), ("----", '-'), (">>>", '>')];

    for &(prefix, ch) in prefixes {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let rest = rest.trim();
            if rest.is_empty() && trimmed.len() >= 8 {
                return Some(DelimiterShape {
                    prefix_char: ch,
                    has_index: false,
                    is_pure_separator: true,
                });
            }
            if has_record_index(rest) {
                return Some(DelimiterShape {
                    prefix_char: ch,
                    has_index: true,
                    is_pure_separator: false,
                });
            }
        }
    }

    None
}

fn has_record_index(rest: &str) -> bool {
    let rest = rest.trim_end_matches(|c: char| c == '=' || c == '-' || c == ' ');
    if rest.is_empty() {
        return false;
    }

    if let Some(after_hash) = rest.strip_prefix('#') {
        return after_hash
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit());
    }

    for label in &["Record ", "Item ", "Entry ", "record ", "item ", "entry "] {
        if let Some(after) = rest.strip_prefix(label) {
            if after.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_hash_boundaries() {
        let output = "header\n\
            ===== #1 =====\nfoo\nbar\n\
            ===== #2 =====\nbaz\nqux\n\
            ===== #3 =====\nquux\n";
        let boundaries = detect_record_boundaries(output).expect("should detect");
        assert_eq!(boundaries.len(), 3);
        assert!(boundaries[0].line.contains("#1"));
        assert!(boundaries[2].line.contains("#3"));
    }

    #[test]
    fn ignores_too_few_boundaries() {
        let output = "===== #1 =====\nfoo\n===== #2 =====\nbar\n";
        assert!(detect_record_boundaries(output).is_none());
    }

    #[test]
    fn detects_pure_separator() {
        let output = "first\n========\nsecond\n========\nthird\n========\nfourth\n";
        let boundaries = detect_record_boundaries(output).expect("should detect");
        assert_eq!(boundaries.len(), 3);
    }

    #[test]
    fn detects_dashes() {
        let output = "---- #1\ndata1\n---- #2\ndata2\n---- #3\ndata3\n";
        let boundaries = detect_record_boundaries(output).expect("should detect");
        assert_eq!(boundaries.len(), 3);
    }

    #[test]
    fn build_segments_splits_correctly() {
        let output = "preamble\n===== #1 =====\nfoo\n===== #2 =====\nbar\n===== #3 =====\nbaz\n";
        let boundaries = detect_record_boundaries(output).unwrap();
        let segments = build_segments(output, &boundaries);

        let boundary_count = segments
            .iter()
            .filter(|s| matches!(s, Segment::Boundary(_)))
            .count();
        assert_eq!(boundary_count, 3);
    }

    #[test]
    fn shape_rejects_mixed_prefix() {
        let s1 = DelimiterShape {
            prefix_char: '=',
            has_index: true,
            is_pure_separator: false,
        };
        let s2 = DelimiterShape {
            prefix_char: '-',
            has_index: true,
            is_pure_separator: false,
        };
        assert!(!s1.matches(&s2));
    }

    #[test]
    fn has_record_index_variations() {
        assert!(has_record_index("#1"));
        assert!(has_record_index("#42 ====="));
        assert!(has_record_index("Record 3"));
        assert!(has_record_index("Item 7"));
        assert!(!has_record_index(""));
        assert!(!has_record_index("random text"));
    }
}
