//! Tamper-evident policy bundles for distributing OCLA policy configuration.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Policy criticality classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyCriticality {
    Critical,
    High,
    Medium,
    Low,
}

/// What happens when a policy expires without renewal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpiryBehavior {
    FailClosed,
    FailOpen,
    GracePeriod,
}

/// Resolved fail mode for runtime decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedFailMode {
    Allow,
    Deny,
    DenyAfterGrace { remaining_seconds: u64 },
}

/// A signed policy bundle containing rules and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBundle {
    pub bundle_id: String,
    pub version: u32,
    pub created_at: String,
    pub rules: Vec<PolicyBundleRule>,
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// A rule within a policy bundle — transport-level representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBundleRule {
    pub rule_id: String,
    pub level: String,
    pub effect: String,
    pub conditions: Value,
    pub priority: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criticality: Option<PolicyCriticality>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry_behavior: Option<ExpiryBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grace_period_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_policy_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_known_good_allowed: Option<bool>,
}

/// Determines the effective fail mode for an expired policy rule.
/// `seconds_since_expiry` is how long ago the policy's signature/TTL expired.
pub fn effective_fail_mode(rule: &PolicyBundleRule, seconds_since_expiry: u64) -> ResolvedFailMode {
    let behavior = rule
        .expiry_behavior
        .as_ref()
        .unwrap_or(&ExpiryBehavior::FailOpen);
    match behavior {
        ExpiryBehavior::FailClosed => ResolvedFailMode::Deny,
        ExpiryBehavior::FailOpen => ResolvedFailMode::Allow,
        ExpiryBehavior::GracePeriod => {
            let grace = rule.grace_period_seconds.unwrap_or(300);
            if seconds_since_expiry < grace {
                ResolvedFailMode::DenyAfterGrace {
                    remaining_seconds: grace - seconds_since_expiry,
                }
            } else {
                ResolvedFailMode::Deny
            }
        }
    }
}

/// Result of verifying a bundle's integrity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleVerification {
    Valid,
    InvalidHash { expected: String, actual: String },
    MissingSignature,
    Unsigned,
}

/// Computes a BLAKE3 hash of a canonical JSON representation of policy rules.
pub fn compute_content_hash(rules: &[PolicyBundleRule]) -> String {
    let rules = Value::Array(
        rules
            .iter()
            .map(|rule| {
                let mut value = Map::new();
                value.insert("rule_id".to_string(), Value::String(rule.rule_id.clone()));
                value.insert("level".to_string(), Value::String(rule.level.clone()));
                value.insert("effect".to_string(), Value::String(rule.effect.clone()));
                value.insert(
                    "conditions".to_string(),
                    canonicalize_json(&rule.conditions),
                );
                value.insert("priority".to_string(), Value::from(rule.priority));
                if let Some(criticality) = &rule.criticality {
                    value.insert(
                        "criticality".to_string(),
                        serde_json::to_value(criticality)
                            .expect("policy criticality always serializes to JSON"),
                    );
                }
                if let Some(expiry_behavior) = &rule.expiry_behavior {
                    value.insert(
                        "expiry_behavior".to_string(),
                        serde_json::to_value(expiry_behavior)
                            .expect("expiry behavior always serializes to JSON"),
                    );
                }
                if let Some(grace_period_seconds) = rule.grace_period_seconds {
                    value.insert(
                        "grace_period_seconds".to_string(),
                        Value::from(grace_period_seconds),
                    );
                }
                if let Some(fallback_policy_ref) = &rule.fallback_policy_ref {
                    value.insert(
                        "fallback_policy_ref".to_string(),
                        Value::String(fallback_policy_ref.clone()),
                    );
                }
                if let Some(last_known_good_allowed) = rule.last_known_good_allowed {
                    value.insert(
                        "last_known_good_allowed".to_string(),
                        Value::Bool(last_known_good_allowed),
                    );
                }
                canonicalize_json(&Value::Object(value))
            })
            .collect(),
    );
    let canonical_json =
        serde_json::to_vec(&rules).expect("policy bundle rules always serialize to JSON");
    blake3::hash(&canonical_json).to_hex().to_string()
}

/// Creates an unsigned version-one bundle with an integrity hash.
pub fn create_bundle(bundle_id: &str, rules: Vec<PolicyBundleRule>) -> PolicyBundle {
    let content_hash = compute_content_hash(&rules);
    PolicyBundle {
        bundle_id: bundle_id.to_string(),
        version: 1,
        created_at: Utc::now().to_rfc3339(),
        rules,
        content_hash,
        signature: None,
    }
}

/// Verifies that a bundle's rules still match its recorded integrity hash.
pub fn verify_bundle(bundle: &PolicyBundle) -> BundleVerification {
    let actual = compute_content_hash(&bundle.rules);
    if bundle.content_hash != actual {
        return BundleVerification::InvalidHash {
            expected: bundle.content_hash.clone(),
            actual,
        };
    }

    match bundle.signature.as_deref() {
        None => BundleVerification::Unsigned,
        Some(signature) if signature.trim().is_empty() => BundleVerification::MissingSignature,
        Some(_) => BundleVerification::Valid,
    }
}

/// Loads a JSON policy bundle and rejects a bundle with a mismatched content hash.
pub fn load_bundle(path: &Path) -> Result<PolicyBundle, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("read policy bundle {}: {error}", path.display()))?;
    let bundle: PolicyBundle = serde_json::from_str(&contents)
        .map_err(|error| format!("parse policy bundle {}: {error}", path.display()))?;

    if let BundleVerification::InvalidHash { expected, actual } = verify_bundle(&bundle) {
        return Err(format!(
            "policy bundle {} has an invalid content hash: expected {expected}, got {actual}",
            path.display()
        ));
    }

    Ok(bundle)
}

/// Saves a policy bundle as pretty-printed JSON.
pub fn save_bundle(bundle: &PolicyBundle, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(bundle)
        .map_err(|error| format!("serialize policy bundle: {error}"))?;
    std::fs::write(path, format!("{json}\n"))
        .map_err(|error| format!("write policy bundle {}: {error}", path.display()))
}

/// Merges overlay rules into a base bundle, preserving base-rule order.
pub fn merge_bundles(base: &PolicyBundle, overlay: &PolicyBundle) -> PolicyBundle {
    let base_rule_ids: HashSet<&str> = base
        .rules
        .iter()
        .map(|rule| rule.rule_id.as_str())
        .collect();
    let overlay_rules: BTreeMap<&str, &PolicyBundleRule> = overlay
        .rules
        .iter()
        .map(|rule| (rule.rule_id.as_str(), rule))
        .collect();
    let mut rules = Vec::with_capacity(base.rules.len() + overlay.rules.len());

    for rule in &base.rules {
        if let Some(overlay_rule) = overlay_rules.get(rule.rule_id.as_str()) {
            rules.push((*overlay_rule).clone());
        } else {
            rules.push(rule.clone());
        }
    }
    for rule in &overlay.rules {
        if !base_rule_ids.contains(rule.rule_id.as_str()) {
            rules.push(rule.clone());
        }
    }

    let mut merged = if overlay.version >= base.version {
        overlay.clone()
    } else {
        base.clone()
    };
    merged.rules = rules;
    merged.content_hash = compute_content_hash(&merged.rules);
    merged
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_json).collect()),
        Value::Object(values) => {
            let sorted: BTreeMap<_, _> = values.iter().collect();
            let mut canonical = Map::new();
            for (key, value) in sorted {
                canonical.insert(key.clone(), canonicalize_json(value));
            }
            Value::Object(canonical)
        }
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{from_value, json, to_value};

    use super::{
        BundleVerification, ExpiryBehavior, PolicyBundleRule, PolicyCriticality, ResolvedFailMode,
        compute_content_hash, create_bundle, effective_fail_mode, load_bundle, merge_bundles,
        save_bundle, verify_bundle,
    };

    fn rule(rule_id: &str, effect: &str) -> PolicyBundleRule {
        PolicyBundleRule {
            rule_id: rule_id.to_string(),
            level: "standard".to_string(),
            effect: effect.to_string(),
            conditions: json!({"source": "local"}),
            priority: 10,
            criticality: None,
            expiry_behavior: None,
            grace_period_seconds: None,
            fallback_policy_ref: None,
            last_known_good_allowed: None,
        }
    }

    #[test]
    fn test_fail_mode_closed() {
        let mut rule = rule("critical", "deny");
        rule.criticality = Some(PolicyCriticality::Critical);
        rule.expiry_behavior = Some(ExpiryBehavior::FailClosed);

        assert_eq!(effective_fail_mode(&rule, 0), ResolvedFailMode::Deny);
    }

    #[test]
    fn test_fail_mode_open() {
        let mut rule = rule("low", "allow");
        rule.criticality = Some(PolicyCriticality::Low);
        rule.expiry_behavior = Some(ExpiryBehavior::FailOpen);

        assert_eq!(effective_fail_mode(&rule, 60), ResolvedFailMode::Allow);
    }

    #[test]
    fn test_fail_mode_grace_period() {
        let mut rule = rule("grace", "deny");
        rule.expiry_behavior = Some(ExpiryBehavior::GracePeriod);
        rule.grace_period_seconds = Some(120);

        assert_eq!(
            effective_fail_mode(&rule, 30),
            ResolvedFailMode::DenyAfterGrace {
                remaining_seconds: 90,
            }
        );
        assert_eq!(effective_fail_mode(&rule, 120), ResolvedFailMode::Deny);
    }

    #[test]
    fn test_backward_compat_no_classification() {
        let value = json!({
            "rule_id": "legacy",
            "level": "standard",
            "effect": "allow",
            "conditions": {},
            "priority": 10
        });
        let rule: PolicyBundleRule = from_value(value).unwrap();

        assert_eq!(effective_fail_mode(&rule, 1), ResolvedFailMode::Allow);
    }

    #[test]
    fn test_serde_roundtrip_with_classification() {
        let mut original = rule("classified", "deny");
        original.criticality = Some(PolicyCriticality::High);
        original.expiry_behavior = Some(ExpiryBehavior::GracePeriod);
        original.grace_period_seconds = Some(600);
        original.fallback_policy_ref = Some("policy://fallback".to_string());
        original.last_known_good_allowed = Some(true);

        let serialized = to_value(&original).unwrap();
        let round_tripped: PolicyBundleRule = from_value(serialized).unwrap();

        assert_eq!(round_tripped.rule_id, original.rule_id);
        assert_eq!(round_tripped.criticality, original.criticality);
        assert_eq!(round_tripped.expiry_behavior, original.expiry_behavior);
        assert_eq!(
            round_tripped.grace_period_seconds,
            original.grace_period_seconds
        );
        assert_eq!(
            round_tripped.fallback_policy_ref,
            original.fallback_policy_ref
        );
        assert_eq!(
            round_tripped.last_known_good_allowed,
            original.last_known_good_allowed
        );
    }

    #[test]
    fn created_bundle_verifies_as_unsigned() {
        let bundle = create_bundle("default", vec![rule("allow-local", "allow")]);

        assert_eq!(verify_bundle(&bundle), BundleVerification::Unsigned);
    }

    #[test]
    fn tampered_rule_has_an_invalid_hash() {
        let mut bundle = create_bundle("default", vec![rule("allow-local", "allow")]);
        bundle.rules[0].effect = "deny".to_string();

        assert!(matches!(
            verify_bundle(&bundle),
            BundleVerification::InvalidHash { .. }
        ));
    }

    #[test]
    fn bundle_with_a_signature_verifies_as_valid() {
        let mut bundle = create_bundle("default", vec![rule("allow-local", "allow")]);
        bundle.signature = Some("signature".to_string());

        assert_eq!(verify_bundle(&bundle), BundleVerification::Valid);
    }

    #[test]
    fn save_and_load_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("policy-bundle.json");
        let bundle = create_bundle("default", vec![rule("allow-local", "allow")]);

        save_bundle(&bundle, &path).unwrap();

        let loaded = load_bundle(&path).unwrap();
        assert_eq!(loaded.bundle_id, bundle.bundle_id);
        assert_eq!(loaded.rules[0].rule_id, bundle.rules[0].rule_id);
        assert_eq!(loaded.content_hash, bundle.content_hash);
    }

    #[test]
    fn merge_replaces_matching_rules_and_appends_new_ones() {
        let base = create_bundle(
            "base",
            vec![rule("allow-local", "allow"), rule("deny-remote", "deny")],
        );
        let mut overlay = create_bundle(
            "overlay",
            vec![rule("allow-local", "deny"), rule("audit", "audit")],
        );
        overlay.version = 2;

        let merged = merge_bundles(&base, &overlay);

        assert_eq!(merged.bundle_id, "overlay");
        assert_eq!(merged.version, 2);
        assert_eq!(merged.rules.len(), 3);
        assert_eq!(merged.rules[0].effect, "deny");
        assert_eq!(merged.rules[1].rule_id, "deny-remote");
        assert_eq!(merged.rules[2].rule_id, "audit");
        assert_eq!(merged.content_hash, compute_content_hash(&merged.rules));
    }

    #[test]
    fn empty_rules_have_a_stable_valid_hash() {
        let mut bundle = create_bundle("empty", Vec::new());
        bundle.signature = Some("signature".to_string());

        assert_eq!(bundle.content_hash, compute_content_hash(&[]));
        assert_eq!(verify_bundle(&bundle), BundleVerification::Valid);
    }

    #[test]
    fn object_key_order_does_not_change_content_hash() {
        let first = rule("ordered", "allow");
        let mut second = first.clone();
        second.conditions = json!({"source": "local", "region": "eu"});
        let mut reordered = second.clone();
        reordered.conditions = json!({"region": "eu", "source": "local"});

        assert_eq!(
            compute_content_hash(&[second]),
            compute_content_hash(&[reordered])
        );
    }
}
