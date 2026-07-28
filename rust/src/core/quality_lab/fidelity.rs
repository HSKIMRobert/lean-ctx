/// Fidelity classification for compressed output.
/// Determines what kind of quality guarantee the compression provides.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum FidelityClassV1 {
    /// Byte-exact reproduction of the original content.
    Exact,
    /// Structural elements preserved (functions, exports, imports, identifiers).
    /// Content may be reformatted or abbreviated.
    Structural,
    /// Meaning preserved but representation changed (summaries, abstractions).
    /// Requires explicit opt-in via policy.
    Lossy,
    /// Fidelity could not be assessed (unknown format, empty content).
    Unknown,
}

/// Result of assessing the fidelity of a compression operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FidelityAssessment {
    pub class: FidelityClassV1,
    pub preservation_score: f64,
    pub identifier_score: f64,
    pub density: f64,
    pub passed_quality_gate: bool,
}

impl FidelityClassV1 {
    pub fn is_lossless(&self) -> bool {
        matches!(self, Self::Exact | Self::Structural)
    }

    pub fn savings_eligible(&self) -> bool {
        matches!(self, Self::Exact | Self::Structural)
    }

    pub fn requires_policy_opt_in(&self) -> bool {
        matches!(self, Self::Lossy)
    }
}

impl FidelityAssessment {
    pub fn token_savings_pct(&self, original_tokens: usize, compressed_tokens: usize) -> f64 {
        if original_tokens == 0 {
            return 0.0;
        }

        (1.0 - compressed_tokens as f64 / original_tokens as f64) * 100.0
    }
}

pub fn assess_fidelity(original: &str, compressed: &str, ext: &str) -> FidelityAssessment {
    if original == compressed {
        return FidelityAssessment {
            class: FidelityClassV1::Exact,
            preservation_score: 1.0,
            identifier_score: 1.0,
            density: 1.0,
            passed_quality_gate: true,
        };
    }

    if original.is_empty() || compressed.is_empty() {
        return unknown_assessment();
    }

    let quality_score = crate::core::quality::score(original, compressed, ext);
    let preservation = crate::core::preservation::measure(original, compressed, ext);
    let preservation_score = preservation.overall();

    let class = classify(&quality_score, preservation_score);

    FidelityAssessment {
        class,
        preservation_score,
        identifier_score: quality_score.identifier_score,
        density: quality_score.density,
        passed_quality_gate: matches!(class, FidelityClassV1::Exact | FidelityClassV1::Structural),
    }
}

fn classify(qs: &crate::core::quality::QualityScore, preservation: f64) -> FidelityClassV1 {
    let has_ast = qs.ast_score > 0.0;

    let structural = if has_ast {
        qs.composite >= 0.75 && preservation >= 0.90
    } else {
        qs.identifier_score >= 0.6 && preservation >= 0.90
    };

    if structural {
        return FidelityClassV1::Structural;
    }

    let signal = if has_ast {
        qs.composite
    } else {
        qs.identifier_score
    };

    if signal >= 0.4 || preservation >= 0.7 {
        return FidelityClassV1::Lossy;
    }
    if signal < 0.3 && preservation < 0.5 {
        return FidelityClassV1::Unknown;
    }
    FidelityClassV1::Lossy
}

pub fn assess_fidelity_from_bytes(
    original: &[u8],
    compressed: &[u8],
    ext: &str,
) -> FidelityAssessment {
    let Ok(original) = std::str::from_utf8(original) else {
        return unknown_assessment();
    };
    let Ok(compressed) = std::str::from_utf8(compressed) else {
        return unknown_assessment();
    };

    assess_fidelity(original, compressed, ext)
}

fn unknown_assessment() -> FidelityAssessment {
    FidelityAssessment {
        class: FidelityClassV1::Unknown,
        preservation_score: 0.0,
        identifier_score: 0.0,
        density: 0.0,
        passed_quality_gate: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{FidelityAssessment, FidelityClassV1, assess_fidelity, assess_fidelity_from_bytes};

    #[test]
    fn test_exact_fidelity_for_identical_content() {
        let source = "pub fn calculate_total() -> f64 { 42.0 }";

        let assessment = assess_fidelity(source, source, "rs");

        assert_eq!(assessment.class, FidelityClassV1::Exact);
        assert_eq!(assessment.preservation_score, 1.0);
        assert!(assessment.passed_quality_gate);
    }

    #[test]
    fn test_structural_fidelity_for_good_compression() {
        let original = r#"
pub fn calculate_total(items: &[Item]) -> f64 {
    items.iter().map(|i| i.price * i.quantity as f64).sum()
}

pub fn format_receipt(total: f64) -> String {
    format!("Total: ${:.2}", total)
}
"#;
        let compressed = r#"pub fn calculate_total(items: &[Item]) -> f64 {
    items.iter()
        .map(|i| i.price * i.quantity as f64)
        .sum()
}
pub fn format_receipt(total: f64) -> String {
    format!("Total: ${total:.2}")
}"#;

        let assessment = assess_fidelity(original, compressed, "rs");

        assert_eq!(assessment.class, FidelityClassV1::Structural);
        assert!(assessment.preservation_score >= 0.95);
        assert!(assessment.passed_quality_gate);
    }

    #[test]
    fn test_lossy_fidelity_for_aggressive_compression() {
        let original = r#"
pub fn calculate_total(items: &[Item]) -> f64 {
    items.iter().map(|i| i.price * i.quantity as f64).sum()
}

pub fn format_receipt(total: f64) -> String {
    format!("Total: ${:.2}", total)
}
"#;
        let compressed = "pub fn calculate_total(items: &[Item]) -> f64 { ... }";

        let assessment = assess_fidelity(original, compressed, "rs");

        assert_eq!(assessment.class, FidelityClassV1::Lossy);
        assert!(!assessment.passed_quality_gate);
    }

    #[test]
    fn test_unknown_fidelity_for_empty_input() {
        let assessment = assess_fidelity("", "summary", "txt");

        assert_eq!(assessment.class, FidelityClassV1::Unknown);
        assert!(!assessment.passed_quality_gate);
    }

    #[test]
    fn test_exact_is_lossless() {
        assert!(FidelityClassV1::Exact.is_lossless());
    }

    #[test]
    fn test_lossy_requires_policy() {
        assert!(FidelityClassV1::Lossy.requires_policy_opt_in());
        assert!(!FidelityClassV1::Structural.requires_policy_opt_in());
    }

    #[test]
    fn test_savings_pct_calculation() {
        let assessment = FidelityAssessment {
            class: FidelityClassV1::Structural,
            preservation_score: 1.0,
            identifier_score: 1.0,
            density: 1.0,
            passed_quality_gate: true,
        };

        assert_eq!(assessment.token_savings_pct(100, 40), 60.0);
        assert_eq!(assessment.token_savings_pct(0, 40), 0.0);
    }

    #[test]
    fn test_bytes_assessment_with_valid_utf8() {
        let source = b"pub fn total() -> usize { 42 }";

        let assessment = assess_fidelity_from_bytes(source, source, "rs");

        assert_eq!(assessment.class, FidelityClassV1::Exact);
    }

    #[test]
    fn test_bytes_assessment_with_invalid_utf8() {
        let assessment = assess_fidelity_from_bytes(&[0xff], b"summary", "txt");

        assert_eq!(assessment.class, FidelityClassV1::Unknown);
        assert!(!assessment.passed_quality_gate);
    }

    #[test]
    fn test_fidelity_ordering() {
        assert!(FidelityClassV1::Exact < FidelityClassV1::Structural);
        assert!(FidelityClassV1::Structural < FidelityClassV1::Lossy);
        assert!(FidelityClassV1::Lossy < FidelityClassV1::Unknown);
    }

    #[test]
    fn test_savings_eligibility() {
        assert!(FidelityClassV1::Exact.savings_eligible());
        assert!(FidelityClassV1::Structural.savings_eligible());
        assert!(!FidelityClassV1::Lossy.savings_eligible());
        assert!(!FidelityClassV1::Unknown.savings_eligible());
    }
}
