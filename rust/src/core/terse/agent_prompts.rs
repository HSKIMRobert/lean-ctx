//! Layer 3: Agent output shaping prompts.
//!
//! Generates prompt instructions that guide the LLM to produce terser output.
//! Four levels, scientifically grounded:
//! - Lite: Brevity constraint (arXiv:2604.00025)
//! - Standard: Telegraph-English-inspired atomic facts
//! - Max: Full TE-style symbolic compression
//! - Adaptive: Thompson sampling across levels (future)

use crate::core::config::CompressionLevel;

/// Generates the agent prompt block for the given compression level.
pub fn build_prompt_block(level: &CompressionLevel) -> String {
    match level {
        CompressionLevel::Off => String::new(),
        CompressionLevel::Lite => LITE_PROMPT.to_string(),
        CompressionLevel::Standard => STANDARD_PROMPT.to_string(),
        CompressionLevel::Max => MAX_PROMPT.to_string(),
    }
}

const LITE_PROMPT: &str = "\
OUTPUT STYLE: concise
- Bullet points over paragraphs
- Skip filler words and hedging (\"I think\", \"probably\", \"it seems\")
- 1-sentence explanations max, then code/action
- No repeating what the user said";

const STANDARD_PROMPT: &str = "\
OUTPUT STYLE: dense
- Each statement = one atomic fact line
- Use abbreviations: fn, cfg, impl, deps, req, res, ctx, err, ret
- Diff lines only (+/-/~), never repeat unchanged code
- Symbols: → (causes), + (adds), − (removes), ~ (modifies), ∴ (therefore)
- No narration, no filler, no hedging
- BUDGET: ≤200 tokens per response unless code block required";

const MAX_PROMPT: &str = "\
OUTPUT STYLE: expert-terse
- Telegraph format: subject-verb-object, drop articles/prepositions
- Symbolic vocabulary: → cause, ∵ because, ∴ therefore, ⊕ add, ⊖ remove, Δ change, ≈ similar, ≠ different, ∈ in/member, ∅ empty/none, ✓ ok, ✗ fail
- Code blocks: untouched (never compress code syntax)
- Each line: max 80 chars
- Zero narration, zero filler
- BUDGET: ≤100 tokens per non-code response";

/// Formats the compression level for inclusion in session/compaction context.
pub fn session_context_tag(level: &CompressionLevel) -> Option<String> {
    if !level.is_active() {
        return None;
    }
    Some(format!("<config compression=\"{}\" />", level.label()))
}

/// Formats a resume block hint for session restore.
pub fn resume_block_hint(level: &CompressionLevel) -> Option<String> {
    match level {
        CompressionLevel::Off => None,
        CompressionLevel::Lite => Some(
            "[COMPRESSION: lite] Keep responses concise. Bullet points, avoid filler.".to_string(),
        ),
        CompressionLevel::Standard => Some(
            "[COMPRESSION: standard] Dense output. Atomic fact lines, abbreviations, diff-only code.".to_string(),
        ),
        CompressionLevel::Max => Some(
            "[COMPRESSION: max] Expert-terse mode. Telegraph format, symbolic vocabulary, zero narration.".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_returns_empty() {
        assert!(build_prompt_block(&CompressionLevel::Off).is_empty());
    }

    #[test]
    fn lite_contains_bullet() {
        let p = build_prompt_block(&CompressionLevel::Lite);
        assert!(p.contains("Bullet"));
    }

    #[test]
    fn standard_contains_abbreviations() {
        let p = build_prompt_block(&CompressionLevel::Standard);
        assert!(p.contains("fn, cfg, impl"));
    }

    #[test]
    fn max_contains_telegraph() {
        let p = build_prompt_block(&CompressionLevel::Max);
        assert!(p.contains("Telegraph"));
    }

    #[test]
    fn session_tag_none_for_off() {
        assert!(session_context_tag(&CompressionLevel::Off).is_none());
    }

    #[test]
    fn session_tag_present_for_active() {
        let tag = session_context_tag(&CompressionLevel::Standard).unwrap();
        assert!(tag.contains("standard"));
    }

    #[test]
    fn resume_hint_none_for_off() {
        assert!(resume_block_hint(&CompressionLevel::Off).is_none());
    }

    #[test]
    fn resume_hint_present_for_max() {
        let hint = resume_block_hint(&CompressionLevel::Max).unwrap();
        assert!(hint.contains("max"));
    }
}
