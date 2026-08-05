use serde::{Deserialize, Serialize};

/// ISO 4217 currency code.
pub type CurrencyCode = String;

/// Sub-cent precision money. NOT floating point.
/// Rounding to legal minor units happens only at invoice time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoneyV1 {
    pub currency: CurrencyCode,
    pub coefficient: i128,
    pub scale: u8,
}

#[cfg(test)]
mod tests {
    use crate::MoneyV1;

    #[test]
    fn serialization_round_trip() {
        let money = MoneyV1 {
            currency: "USD".to_owned(),
            coefficient: 12345,
            scale: 4,
        };
        let json = serde_json::to_string(&money).expect("money should serialize");
        let decoded: MoneyV1 = serde_json::from_str(&json).expect("money should deserialize");
        assert_eq!(decoded, money);
    }

    #[test]
    fn coefficient_and_scale_represent_sub_cent_value() {
        let money = MoneyV1 {
            currency: "USD".to_owned(),
            coefficient: 1,
            scale: 3,
        };
        assert_eq!(money.coefficient, 1);
        assert_eq!(money.scale, 3);
    }
}
