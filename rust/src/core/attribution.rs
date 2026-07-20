//! Deterministic identifiers and grouping for savings attribution records.

/// Generate a stable, compact identifier for an attribution record.
pub fn generate_attribution_id(
    tool: &str,
    mechanism: &str,
    session_id: &str,
    timestamp: &str,
) -> String {
    let identity = format!("{tool}|{mechanism}|{session_id}|{timestamp}");
    let digest = blake3::hash(identity.as_bytes()).to_hex();
    digest[..16].to_owned()
}

/// Return whether `new_id` has not already been booked.
pub fn check_unique(existing_ids: &[&str], new_id: &str) -> bool {
    !existing_ids.contains(&new_id)
}

/// Map a savings mechanism to its canonical attribution group.
pub fn attribution_group_for_mechanism(mechanism: &str) -> &'static str {
    match mechanism {
        "compression" => "input_optimization",
        "routing" => "model_selection",
        "caching" => "cache_discount",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::{attribution_group_for_mechanism, check_unique, generate_attribution_id};

    #[test]
    fn generation_is_deterministic() {
        let first = generate_attribution_id("tool", "compression", "session", "ts");
        let second = generate_attribution_id("tool", "compression", "session", "ts");

        assert_eq!(first, second);
    }

    #[test]
    fn changing_identity_changes_id() {
        let original = generate_attribution_id("tool", "compression", "session", "ts");
        let changed = generate_attribution_id("other-tool", "compression", "session", "ts");

        assert_ne!(original, changed);
    }

    #[test]
    fn uniqueness_rejects_existing_id() {
        assert!(check_unique(&["abc", "def"], "ghi"));
        assert!(!check_unique(&["abc", "def"], "def"));
    }

    #[test]
    fn mechanisms_map_to_expected_groups() {
        assert_eq!(
            attribution_group_for_mechanism("compression"),
            "input_optimization"
        );
        assert_eq!(
            attribution_group_for_mechanism("routing"),
            "model_selection"
        );
        assert_eq!(attribution_group_for_mechanism("caching"), "cache_discount");
        assert_eq!(attribution_group_for_mechanism("other"), "other");
    }

    #[test]
    fn generated_id_is_a_16_character_hex_digest() {
        let id = generate_attribution_id("tool", "routing", "session", "ts");

        assert_eq!(id.len(), 16);
        assert!(id.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn distinct_records_have_collision_resistant_ids() {
        let first = generate_attribution_id("tool", "routing", "session-a", "ts");
        let second = generate_attribution_id("tool", "routing", "session-b", "ts");
        let third = generate_attribution_id("tool", "routing", "session-a", "ts+1");

        assert_ne!(first, second);
        assert_ne!(first, third);
        assert_ne!(second, third);
    }
}
