use serde::{Deserialize, Serialize};

/// Granular token usage breakdown.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageBreakdownV1 {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_write_tokens: u64,
    pub reasoning_tokens: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_units: Vec<MeasuredUnitV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasuredUnitV1 {
    pub unit_type: String,
    pub quantity: u64,
}

#[cfg(test)]
mod tests {
    use crate::{MeasuredUnitV1, UsageBreakdownV1};

    #[test]
    fn serialization_round_trip() {
        let usage = UsageBreakdownV1 {
            input_tokens: 10,
            output_tokens: 5,
            cached_input_tokens: 3,
            cache_write_tokens: 2,
            reasoning_tokens: 1,
            other_units: vec![MeasuredUnitV1 {
                unit_type: "image".to_owned(),
                quantity: 1,
            }],
        };
        let json = serde_json::to_string(&usage).expect("usage should serialize");
        let decoded = serde_json::from_str(&json).expect("usage should deserialize");
        assert_eq!(usage, decoded);
    }
}
