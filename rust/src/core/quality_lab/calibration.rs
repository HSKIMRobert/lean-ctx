use crate::core::tokens::{TokenizerFamily, count_tokens_for, detect_tokenizer};

/// Accuracy level of a calibrated token count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CalibrationAccuracy {
    /// Exact BPE encoding match (e.g., o200k_base for GPT-4o)
    ExactTextEncoding,
    /// Proxy tokenizer with empirical correction factor applied
    CorrectedProxyTokenizer,
    /// Proxy tokenizer without correction (approximate)
    ProxyTokenizer,
    /// Character-based fallback (4 chars/token estimate)
    CharFallback,
}

impl CalibrationAccuracy {
    const fn rank(self) -> u8 {
        match self {
            Self::ExactTextEncoding => 3,
            Self::CorrectedProxyTokenizer => 2,
            Self::ProxyTokenizer => 1,
            Self::CharFallback => 0,
        }
    }
}

impl Ord for CalibrationAccuracy {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl PartialOrd for CalibrationAccuracy {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Token count with calibration metadata.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CalibratedCount {
    pub tokens: u64,
    #[serde(
        serialize_with = "serialize_tokenizer_family",
        deserialize_with = "deserialize_tokenizer_family"
    )]
    pub family: TokenizerFamily,
    pub accuracy: CalibrationAccuracy,
}

/// Calibration data for a tokenizer family.
#[derive(Debug, Clone)]
pub struct CalibrationProfile {
    pub family: TokenizerFamily,
    pub correction_factor: f64,
    pub accuracy: CalibrationAccuracy,
    pub variance_pct: f64,
}

/// Returns the built-in calibration profile for a tokenizer family.
pub fn calibration_profile(family: TokenizerFamily) -> CalibrationProfile {
    match family {
        TokenizerFamily::O200kBase => CalibrationProfile {
            family,
            correction_factor: 1.0,
            accuracy: CalibrationAccuracy::ExactTextEncoding,
            variance_pct: 0.0,
        },
        TokenizerFamily::Cl100k => CalibrationProfile {
            family,
            correction_factor: 1.0,
            accuracy: CalibrationAccuracy::CorrectedProxyTokenizer,
            variance_pct: 3.0,
        },
        TokenizerFamily::Gemini => CalibrationProfile {
            family,
            correction_factor: 1.08,
            accuracy: CalibrationAccuracy::CorrectedProxyTokenizer,
            variance_pct: 5.0,
        },
        TokenizerFamily::Llama => CalibrationProfile {
            family,
            correction_factor: 1.0,
            accuracy: CalibrationAccuracy::ProxyTokenizer,
            variance_pct: 8.0,
        },
    }
}

/// Counts tokens and applies the tokenizer family's calibration profile.
pub fn count_tokens_with_calibration(text: &str, family: TokenizerFamily) -> CalibratedCount {
    if text.is_empty() {
        return CalibratedCount {
            tokens: 0,
            family,
            accuracy: CalibrationAccuracy::ExactTextEncoding,
        };
    }

    let profile = calibration_profile(family);
    let raw = count_tokens_for(text, family);
    let corrected = (raw as f64 * profile.correction_factor).ceil() as u64;

    CalibratedCount {
        tokens: corrected,
        family,
        accuracy: profile.accuracy,
    }
}

/// Detects the tokenizer family from a model name and returns a calibrated count.
pub fn count_tokens_calibrated_from_model(text: &str, model: &str) -> CalibratedCount {
    count_tokens_with_calibration(text, detect_tokenizer(model))
}

/// Returns inclusive lower and upper bounds for a calibrated count.
pub fn calibration_variance(count: &CalibratedCount) -> (u64, u64) {
    let variance_pct = calibration_profile(count.family).variance_pct;
    let variance = count.tokens as f64 * variance_pct / 100.0;
    let lower = (count.tokens as f64 - variance).floor().max(0.0) as u64;
    let upper = (count.tokens as f64 + variance).ceil() as u64;
    (lower, upper)
}

/// Returns calibrated counts for every supported tokenizer family.
pub fn compare_calibration(text: &str) -> Vec<CalibratedCount> {
    [
        TokenizerFamily::O200kBase,
        TokenizerFamily::Cl100k,
        TokenizerFamily::Gemini,
        TokenizerFamily::Llama,
    ]
    .into_iter()
    .map(|family| count_tokens_with_calibration(text, family))
    .collect()
}

fn serialize_tokenizer_family<S>(family: &TokenizerFamily, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(match family {
        TokenizerFamily::O200kBase => "O200kBase",
        TokenizerFamily::Cl100k => "Cl100k",
        TokenizerFamily::Gemini => "Gemini",
        TokenizerFamily::Llama => "Llama",
    })
}

fn deserialize_tokenizer_family<'de, D>(deserializer: D) -> Result<TokenizerFamily, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = <String as serde::Deserialize>::deserialize(deserializer)?;
    match value.as_str() {
        "O200kBase" => Ok(TokenizerFamily::O200kBase),
        "Cl100k" => Ok(TokenizerFamily::Cl100k),
        "Gemini" => Ok(TokenizerFamily::Gemini),
        "Llama" => Ok(TokenizerFamily::Llama),
        _ => Err(serde::de::Error::custom("unknown tokenizer family")),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CalibratedCount, CalibrationAccuracy, calibration_profile, calibration_variance,
        compare_calibration, count_tokens_calibrated_from_model, count_tokens_with_calibration,
    };
    use crate::core::tokens::{TokenizerFamily, count_tokens_for};

    const SAMPLE: &str = "fn summarize(records: &[Record]) -> Summary { records.iter().collect() }";

    #[test]
    fn test_o200k_exact_calibration() {
        let profile = calibration_profile(TokenizerFamily::O200kBase);
        let count = count_tokens_with_calibration(SAMPLE, TokenizerFamily::O200kBase);

        assert_eq!(profile.correction_factor, 1.0);
        assert_eq!(count.accuracy, CalibrationAccuracy::ExactTextEncoding);
        assert_eq!(
            count.tokens,
            count_tokens_for(SAMPLE, TokenizerFamily::O200kBase) as u64
        );
    }

    #[test]
    fn test_gemini_correction_factor() {
        let raw = count_tokens_for(SAMPLE, TokenizerFamily::Gemini);
        let count = count_tokens_with_calibration(SAMPLE, TokenizerFamily::Gemini);

        assert_eq!(count.tokens, (raw as f64 * 1.08).ceil() as u64);
    }

    #[test]
    fn test_cl100k_corrected_proxy() {
        let count = count_tokens_with_calibration(SAMPLE, TokenizerFamily::Cl100k);

        assert_eq!(count.accuracy, CalibrationAccuracy::CorrectedProxyTokenizer);
    }

    #[test]
    fn test_llama_proxy_accuracy() {
        let count = count_tokens_with_calibration(SAMPLE, TokenizerFamily::Llama);

        assert_eq!(count.accuracy, CalibrationAccuracy::ProxyTokenizer);
    }

    #[test]
    fn test_empty_text_zero_tokens() {
        let count = count_tokens_with_calibration("", TokenizerFamily::Gemini);

        assert_eq!(count.tokens, 0);
        assert_eq!(count.accuracy, CalibrationAccuracy::ExactTextEncoding);
    }

    #[test]
    fn test_model_detection_claude() {
        let count = count_tokens_calibrated_from_model(SAMPLE, "claude-sonnet-4-20250514");

        assert_eq!(count.family, TokenizerFamily::Cl100k);
    }

    #[test]
    fn test_model_detection_gpt4o() {
        let count = count_tokens_calibrated_from_model(SAMPLE, "gpt-4o");

        assert_eq!(count.family, TokenizerFamily::O200kBase);
    }

    #[test]
    fn test_variance_bounds() {
        let count = CalibratedCount {
            tokens: 1_000,
            family: TokenizerFamily::Gemini,
            accuracy: CalibrationAccuracy::CorrectedProxyTokenizer,
        };

        assert_eq!(calibration_variance(&count), (950, 1_050));
    }

    #[test]
    fn test_compare_all_families() {
        let counts = compare_calibration(SAMPLE);
        let families: Vec<TokenizerFamily> = counts.into_iter().map(|count| count.family).collect();

        assert_eq!(
            families,
            vec![
                TokenizerFamily::O200kBase,
                TokenizerFamily::Cl100k,
                TokenizerFamily::Gemini,
                TokenizerFamily::Llama,
            ]
        );
    }

    #[test]
    fn test_calibration_ordering() {
        assert!(
            CalibrationAccuracy::ExactTextEncoding > CalibrationAccuracy::CorrectedProxyTokenizer
        );
        assert!(CalibrationAccuracy::CorrectedProxyTokenizer > CalibrationAccuracy::ProxyTokenizer);
        assert!(CalibrationAccuracy::ProxyTokenizer > CalibrationAccuracy::CharFallback);
    }
}
