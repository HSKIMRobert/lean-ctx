//! Output sanitizer: detects and cleans degenerate model artifacts from compressed output.
//!
//! Catches CJK runs, repeated-symbol floods, and other non-informational sequences
//! that downstream summarizer models can produce when they fail to parse dense
//! symbolic/compressed input (see GitHub #257).

/// Returns true if the character belongs to CJK Unified Ideographs or common CJK ranges.
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // CJK Extension A
        | '\u{F900}'..='\u{FAFF}' // CJK Compatibility Ideographs
        | '\u{2E80}'..='\u{2EFF}' // CJK Radicals Supplement
        | '\u{3000}'..='\u{303F}' // CJK Symbols and Punctuation
        | '\u{31F0}'..='\u{31FF}' // Katakana Phonetic Extensions
        | '\u{3200}'..='\u{32FF}' // Enclosed CJK Letters
        | '\u{FE30}'..='\u{FE4F}' // CJK Compatibility Forms
        | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
        | '\u{1100}'..='\u{11FF}' // Hangul Jamo
    )
}

/// Returns true if a line contains degenerate CJK content:
/// - 3+ consecutive CJK chars in a predominantly non-CJK line, OR
/// - Any CJK chars combined with a symbol flood (classic degenerate pattern)
fn has_degenerate_cjk_run(line: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let total = chars.len();
    if total == 0 {
        return false;
    }

    let cjk_count = chars.iter().filter(|c| is_cjk(**c)).count();
    if cjk_count == 0 {
        return false;
    }

    let cjk_ratio = cjk_count as f64 / total as f64;

    // If the line is >60% CJK, assume it's genuine CJK content (e.g. Chinese docs).
    if cjk_ratio > 0.6 {
        return false;
    }

    // CJK chars + symbol flood = degenerate output (e.g. "肛裂!!!!!!")
    if cjk_count >= 1 && is_symbol_flood(line) {
        return true;
    }

    // CJK + repeated non-alphanumeric (5+) = degenerate even below flood threshold
    if cjk_count >= 1 && has_repeated_symbol(line, 5) {
        return true;
    }

    // 3+ consecutive CJK chars in a predominantly non-CJK line
    let mut consecutive = 0u32;
    for &c in &chars {
        if is_cjk(c) {
            consecutive += 1;
            if consecutive >= 3 {
                return true;
            }
        } else {
            consecutive = 0;
        }
    }
    false
}

/// Returns true if the line has N+ consecutive identical non-alphanumeric chars.
fn has_repeated_symbol(line: &str, threshold: u32) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let mut run = 1u32;
    for i in 1..chars.len() {
        if chars[i] == chars[i - 1] && !chars[i].is_alphanumeric() && chars[i] != ' ' {
            run += 1;
            if run >= threshold {
                return true;
            }
        } else {
            run = 1;
        }
    }
    false
}

/// Returns true if a line is a "symbol flood" — 10+ of the same character repeated.
fn is_symbol_flood(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 10 {
        return false;
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let mut max_run = 1u32;
    let mut current_run = 1u32;
    for i in 1..chars.len() {
        if chars[i] == chars[i - 1] && !chars[i].is_alphanumeric() && chars[i] != ' ' {
            current_run += 1;
            if current_run > max_run {
                max_run = current_run;
            }
        } else {
            current_run = 1;
        }
    }
    max_run >= 10
}

/// Sanitize tool output by removing degenerate lines.
///
/// This is the last-pass filter before output reaches the client.
/// It removes lines that contain degenerate CJK artifacts or symbol floods,
/// which can appear when upstream compression produces content that confuses
/// downstream summarizer models.
pub fn sanitize(output: &str) -> String {
    if output.is_empty() {
        return output.to_string();
    }

    let mut cleaned = Vec::new();
    let mut removed = 0usize;

    for line in output.lines() {
        if has_degenerate_cjk_run(line) || is_symbol_flood(line) {
            removed += 1;
            continue;
        }
        cleaned.push(line);
    }

    if removed == 0 {
        return output.to_string();
    }

    let result = cleaned.join("\n");
    if removed > 0 {
        tracing::debug!("[sanitizer] removed {removed} degenerate line(s) from output");
    }
    result
}

/// Replaces Unicode mathematical/symbolic characters with ASCII equivalents.
/// Used to produce output that is friendly to lightweight downstream models
/// (e.g. Cursor's Thought summarizer) which may degenerate on dense Unicode.
pub fn ascii_safe_symbols(text: &str) -> String {
    text.replace('→', "->")
        .replace('←', "<-")
        .replace('∴', ":.")
        .replace('≈', "~=")
        .replace('≠', "!=")
        .replace('∈', "in")
        .replace('∅', "(none)")
        .replace('⊕', "+")
        .replace('⊖', "-")
        .replace('Δ', "delta")
        .replace('✓', "ok")
        .replace('✗', "FAIL")
        .replace('⚠', "WARN")
        .replace('\u{2192}', "->") // → as used in savings footer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_passes_normal_english() {
        let input = "fn main() {\n    println!(\"hello\");\n}";
        assert_eq!(sanitize(input), input);
    }

    #[test]
    fn clean_removes_degenerate_cjk_in_english() {
        let input = "Explored 22 files, 14 searches\n肛裂!!!!!!!!!!!!!!!!!!\nExploring >";
        let cleaned = sanitize(input);
        assert!(!cleaned.contains("肛裂"));
        assert!(cleaned.contains("Explored 22"));
        assert!(cleaned.contains("Exploring"));
    }

    #[test]
    fn clean_preserves_genuine_cjk_content() {
        let input = "这是一个正常的中文文档，包含完整的句子结构。";
        assert_eq!(sanitize(input), input);
    }

    #[test]
    fn clean_preserves_mixed_cjk_english_docs() {
        let input = "The function 関数 is documented in 文档 for reference.";
        // Only 2 CJK chars per run, should not be flagged
        assert_eq!(sanitize(input), input);
    }

    #[test]
    fn clean_removes_symbol_flood() {
        let input = "normal line\n!!!!!!!!!!!!!!!!!!!!!!!\nanother line";
        let cleaned = sanitize(input);
        assert!(!cleaned.contains("!!!!!!!!!!!!"));
        assert!(cleaned.contains("normal line"));
        assert!(cleaned.contains("another line"));
    }

    #[test]
    fn clean_preserves_normal_punctuation() {
        let input = "Error: something failed!!";
        assert_eq!(sanitize(input), input);
    }

    #[test]
    fn ascii_safe_replaces_unicode_symbols() {
        let out = ascii_safe_symbols("fn → result ✓ or ✗");
        assert_eq!(out, "fn -> result ok or FAIL");
    }

    #[test]
    fn ascii_safe_replaces_math_symbols() {
        let out = ascii_safe_symbols("A ≠ B, C ≈ D, x ∈ set, ∅");
        assert_eq!(out, "A != B, C ~= D, x in set, (none)");
    }

    #[test]
    fn degenerate_cjk_mixed_with_exclamation() {
        assert!(has_degenerate_cjk_run("肛裂!!!!!!"));
        assert!(has_degenerate_cjk_run("result: 乱码输 garbled"));
    }

    #[test]
    fn genuine_cjk_line_not_flagged() {
        assert!(!has_degenerate_cjk_run("这是完整的中文内容，不是乱码"));
    }

    #[test]
    fn short_cjk_pair_not_flagged() {
        assert!(!has_degenerate_cjk_run("the 変数 variable"));
    }

    #[test]
    fn empty_input() {
        assert_eq!(sanitize(""), "");
    }

    #[test]
    fn symbol_flood_exact_threshold() {
        assert!(!is_symbol_flood("!!!!!!!!!")); // 9 — below threshold
        assert!(is_symbol_flood("!!!!!!!!!!")); // 10 — at threshold
    }
}
