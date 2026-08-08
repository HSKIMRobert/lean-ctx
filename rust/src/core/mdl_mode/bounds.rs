//! MDL description-length bounds for structural compression ratios.
//!
//! Formalizes model cost plus data cost in bits for comparing structural
//! descriptions against original token payloads.

use super::structural::StructuralDescription;

/// Compute description length in bits (MDL formalization).
///
/// `DL(desc) = model_cost + data_cost`, where model cost captures structural
/// overhead and data cost approximates token encoding against an LLM vocabulary.
pub(crate) fn description_length(desc: &StructuralDescription) -> f64 {
    let model_cost = ((desc.types.len() + desc.functions.len() + 1) as f64).log2();
    let vocab_size: f64 = 50_000.0;
    let data_cost = desc.description_tokens as f64 * vocab_size.log2();
    model_cost + data_cost
}
#[cfg(test)]
mod tests {
    use super::description_length;
    use crate::core::mdl_mode::structural::generate_structural_description;

    #[test]
    fn description_length_positive_for_nonempty() {
        let desc = generate_structural_description("pub fn answer() -> u8 { 42 }", "lib.rs", "rs");

        assert!(description_length(&desc) > 0.0);
    }

    #[test]
    fn description_length_zero_for_empty() {
        let desc = generate_structural_description("", "empty.rs", "rs");

        assert_eq!(description_length(&desc), 0.0);
    }
}
