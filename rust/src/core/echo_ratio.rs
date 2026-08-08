//! Echo Ratio computation — output-to-input token ratio.
//!
//! Measures what fraction of LLM output tokens were already present in the input.
//! A proxy for compression effectiveness: high echo = agent is repeating context.

use std::collections::HashSet;

use crate::core::tokens::count_tokens;

/// Echo ratio analysis result.
#[derive(Debug, Clone)]
pub struct EchoRatioReport {
    /// Input tokens (context provided).
    pub input_tokens: usize,
    /// Output tokens (LLM response).
    pub output_tokens: usize,
    /// Echo tokens (output tokens that appeared in input).
    pub echo_tokens: usize,
    /// Echo ratio: echo_tokens / output_tokens (0.0-1.0).
    pub ratio: f64,
    /// Verdict: "low" (<0.3), "moderate" (0.3-0.6), "high" (>0.6).
    pub verdict: &'static str,
}

/// Compute echo ratio between input context and LLM output.
/// Uses simple token overlap as a lightweight proxy for semantic echo.
pub fn compute_echo_ratio(input: &str, output: &str) -> EchoRatioReport {
    let input_tokens_count = count_tokens(input);
    let output_tokens_count = count_tokens(output);

    if output_tokens_count == 0 {
        return EchoRatioReport {
            input_tokens: input_tokens_count,
            output_tokens: 0,
            echo_tokens: 0,
            ratio: 0.0,
            verdict: "low",
        };
    }

    // Word-level overlap is a lightweight proxy for exact tokenizer matching.
    let input_words: HashSet<&str> = input.split_whitespace().collect();
    let output_words: Vec<&str> = output.split_whitespace().collect();
    let echo_count = output_words
        .iter()
        .filter(|word| input_words.contains(*word))
        .count();
    let ratio = echo_count as f64 / output_words.len().max(1) as f64;

    let verdict = if ratio < 0.3 {
        "low"
    } else if ratio < 0.6 {
        "moderate"
    } else {
        "high"
    };

    EchoRatioReport {
        input_tokens: input_tokens_count,
        output_tokens: output_tokens_count,
        echo_tokens: echo_count,
        ratio,
        verdict,
    }
}

impl EchoRatioReport {
    /// Render as one-line summary.
    pub fn summary(&self) -> String {
        format!(
            "Echo: {:.0}% ({}) — {}/{} tokens echoed",
            self.ratio * 100.0,
            self.verdict,
            self.echo_tokens,
            self.output_tokens,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_output_gives_zero_ratio() {
        let report = compute_echo_ratio("input context", "");

        assert_eq!(report.output_tokens, 0);
        assert_eq!(report.echo_tokens, 0);
        assert_eq!(report.ratio, 0.0);
        assert_eq!(report.verdict, "low");
    }

    #[test]
    fn no_overlap_gives_zero_ratio() {
        let report = compute_echo_ratio("alpha beta", "gamma delta");

        assert_eq!(report.echo_tokens, 0);
        assert_eq!(report.ratio, 0.0);
        assert_eq!(report.verdict, "low");
    }

    #[test]
    fn complete_echo_gives_high_ratio() {
        let report = compute_echo_ratio("alpha beta", "alpha beta");

        assert_eq!(report.echo_tokens, 2);
        assert_eq!(report.ratio, 1.0);
        assert_eq!(report.verdict, "high");
    }

    #[test]
    fn partial_echo_gives_moderate_ratio() {
        let report = compute_echo_ratio("alpha beta", "alpha gamma");

        assert_eq!(report.echo_tokens, 1);
        assert_eq!(report.ratio, 0.5);
        assert_eq!(report.verdict, "moderate");
    }

    #[test]
    fn summary_format() {
        let report = compute_echo_ratio("alpha beta", "alpha beta");

        assert_eq!(report.summary(), "Echo: 100% (high) — 2/2 tokens echoed");
    }
}
