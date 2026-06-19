//! Org-floor merge (GL #674) — fold a central org policy *underneath* the local
//! project pack so the local pack can only ever **tighten**, never weaken it.
//!
//! This is what makes central distribution un-bypassable: a user editing
//! `.lean-ctx/policy.toml` cannot drop an org deny, replace an org redaction
//! pattern, raise a token budget above the org cap or downgrade a filter action.
//! Every field merges toward the **stricter** side:
//!
//! | Field | Merge |
//! |---|---|
//! | `deny_tools` | union (org ∪ local) |
//! | `allow_tools` | intersection when both set (an allowlist can only narrow) |
//! | `redaction` | union; **org wins** on a name clash (its patterns are fixed) |
//! | `filters.*` actions | the stricter action (`off`<`warn`<`redact`<`block`) |
//! | `blocked_labels` | union |
//! | `egress.forbidden_patterns` | union |
//! | `egress.block_secrets` | `true` wins |
//! | `egress.max_writes_per_min` | the smaller cap |
//! | `max_context_tokens` | the smaller cap |
//! | `audit_retention_days` | the larger window |
//! | `default_read_mode` | org pins it when set, else local |
//!
//! The result is a normal [`ResolvedPolicy`] the runtime enforces exactly like a
//! single pack, so nothing downstream needs to know a floor was applied.

use crate::core::input_filters::FilterAction;
use crate::core::policy::{EgressRules, FilterRules, ResolvedPolicy};

/// Merge a central org policy (`org`, the floor) with the optional local project
/// pack (`local`). With no local pack the org policy is the effective policy on
/// its own; with one, every field is folded toward the stricter side.
#[must_use]
pub fn merge_floor(org: &ResolvedPolicy, local: Option<&ResolvedPolicy>) -> ResolvedPolicy {
    let Some(local) = local else {
        return org.clone();
    };

    // Composition trail, base-most first: org lineage, the org pack, then the
    // local pack's own lineage. The local pack name stays the effective title.
    let mut chain = org.chain.clone();
    if !chain.contains(&org.name) {
        chain.push(org.name.clone());
    }
    for parent in &local.chain {
        if !chain.contains(parent) {
            chain.push(parent.clone());
        }
    }

    let deny_tools = union(&org.deny_tools, &local.deny_tools);

    let mut redaction = local.redaction.clone();
    for (name, pattern) in &org.redaction {
        // org wins: its floor patterns can never be replaced by the local pack.
        redaction.insert(name.clone(), pattern.clone());
    }

    let filters = FilterRules {
        pii: stricter_action(org.filters.pii.as_ref(), local.filters.pii.as_ref()),
        classification: stricter_action(
            org.filters.classification.as_ref(),
            local.filters.classification.as_ref(),
        ),
        injection: stricter_action(
            org.filters.injection.as_ref(),
            local.filters.injection.as_ref(),
        ),
        blocked_labels: union(&org.filters.blocked_labels, &local.filters.blocked_labels),
    };

    let egress = EgressRules {
        forbidden_patterns: union(
            &org.egress.forbidden_patterns,
            &local.egress.forbidden_patterns,
        ),
        block_secrets: merge_block_secrets(org.egress.block_secrets, local.egress.block_secrets),
        max_writes_per_min: min_opt(
            org.egress.max_writes_per_min,
            local.egress.max_writes_per_min,
        ),
    };

    ResolvedPolicy {
        name: local.name.clone(),
        version: local.version.clone(),
        description: format!(
            "{} · org floor: {} v{}",
            local.description, org.name, org.version
        ),
        chain,
        // org pins the read mode when it sets one; otherwise the local choice.
        default_read_mode: org
            .default_read_mode
            .clone()
            .or_else(|| local.default_read_mode.clone()),
        allow_tools: merge_allow(
            org.allow_tools.as_deref(),
            local.allow_tools.as_deref(),
            &deny_tools,
        ),
        deny_tools,
        max_context_tokens: min_opt(org.max_context_tokens, local.max_context_tokens),
        audit_retention_days: max_opt(org.audit_retention_days, local.audit_retention_days),
        redaction,
        filters,
        egress,
    }
}

/// Order-preserving union: every element of `a`, then any new element of `b`.
fn union(a: &[String], b: &[String]) -> Vec<String> {
    let mut out = a.to_vec();
    for item in b {
        if !out.contains(item) {
            out.push(item.clone());
        }
    }
    out
}

/// Intersect two allowlists (each one a capability ceiling). When only one side
/// sets an allowlist that side wins; the result excludes any denied tool so the
/// resolved view never lists a tool the deny list already blocks.
fn merge_allow(
    org: Option<&[String]>,
    local: Option<&[String]>,
    deny: &[String],
) -> Option<Vec<String>> {
    let allow = match (org, local) {
        (Some(o), Some(l)) => o.iter().filter(|t| l.contains(t)).cloned().collect(),
        (Some(o), None) => o.to_vec(),
        (None, Some(l)) => l.to_vec(),
        (None, None) => return None,
    };
    Some(allow.into_iter().filter(|t| !deny.contains(t)).collect())
}

/// Keep the stricter of two filter actions (`block` > `redact` > `warn` >
/// `off`). Tokens are normalised to canonical form on the way out.
fn stricter_action(a: Option<&String>, b: Option<&String>) -> Option<String> {
    let parsed = |s: Option<&String>| s.and_then(|v| FilterAction::parse(v));
    match (parsed(a), parsed(b)) {
        (Some(x), Some(y)) => Some(
            if x.rank() >= y.rank() { x } else { y }
                .as_str()
                .to_string(),
        ),
        (Some(x), None) => Some(x.as_str().to_string()),
        (None, Some(y)) => Some(y.as_str().to_string()),
        (None, None) => None,
    }
}

/// `true` wins (block secrets if *either* side asks for it); otherwise an
/// explicit `false` is preserved; `None` only when neither side states a value.
fn merge_block_secrets(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), _) | (_, Some(false)) => Some(false),
        (None, None) => None,
    }
}

fn min_opt<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

fn max_opt<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn rp(name: &str) -> ResolvedPolicy {
        ResolvedPolicy {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: name.to_string(),
            chain: vec![],
            default_read_mode: None,
            allow_tools: None,
            deny_tools: vec![],
            max_context_tokens: None,
            audit_retention_days: None,
            redaction: BTreeMap::new(),
            filters: FilterRules::default(),
            egress: EgressRules::default(),
        }
    }

    #[test]
    fn no_local_returns_org_unchanged() {
        let mut org = rp("org");
        org.deny_tools = vec!["ctx_shell".into()];
        let merged = merge_floor(&org, None);
        assert_eq!(merged.deny_tools, vec!["ctx_shell".to_string()]);
        assert_eq!(merged.name, "org");
    }

    #[test]
    fn denies_are_unioned_local_cannot_drop_org_deny() {
        let mut org = rp("org");
        org.deny_tools = vec!["ctx_url_read".into()];
        let mut local = rp("local");
        local.deny_tools = vec!["ctx_shell".into()]; // does NOT include the org deny
        let m = merge_floor(&org, Some(&local));
        assert!(m.deny_tools.contains(&"ctx_url_read".to_string()));
        assert!(m.deny_tools.contains(&"ctx_shell".to_string()));
    }

    #[test]
    fn allowlist_intersects_and_drops_denied() {
        let mut org = rp("org");
        org.allow_tools = Some(vec![
            "ctx_read".into(),
            "ctx_search".into(),
            "ctx_shell".into(),
        ]);
        org.deny_tools = vec!["ctx_shell".into()];
        let mut local = rp("local");
        local.allow_tools = Some(vec!["ctx_read".into(), "ctx_edit".into()]);
        let m = merge_floor(&org, Some(&local));
        // Intersection of {read,search,shell} and {read,edit} = {read}; shell
        // is denied anyway. ctx_edit is NOT in the org ceiling → excluded.
        assert_eq!(m.allow_tools, Some(vec!["ctx_read".to_string()]));
    }

    #[test]
    fn org_redaction_pattern_wins_on_clash() {
        let mut org = rp("org");
        org.redaction.insert("secret".into(), "ORG-\\d+".into());
        let mut local = rp("local");
        local.redaction.insert("secret".into(), "weak".into());
        local.redaction.insert("extra".into(), "EX-\\d+".into());
        let m = merge_floor(&org, Some(&local));
        assert_eq!(m.redaction.get("secret").unwrap(), "ORG-\\d+");
        assert_eq!(m.redaction.get("extra").unwrap(), "EX-\\d+");
    }

    #[test]
    fn filter_action_keeps_stricter() {
        let mut org = rp("org");
        org.filters.pii = Some("block".into());
        org.filters.injection = Some("warn".into());
        let mut local = rp("local");
        local.filters.pii = Some("redact".into()); // weaker → org wins
        local.filters.injection = Some("block".into()); // stronger → local wins
        let m = merge_floor(&org, Some(&local));
        assert_eq!(m.filters.pii.as_deref(), Some("block"));
        assert_eq!(m.filters.injection.as_deref(), Some("block"));
    }

    #[test]
    fn caps_take_stricter_side() {
        let mut org = rp("org");
        org.max_context_tokens = Some(8000);
        org.audit_retention_days = Some(365);
        org.egress.max_writes_per_min = Some(10);
        org.egress.block_secrets = Some(true);
        let mut local = rp("local");
        local.max_context_tokens = Some(20000); // org's smaller cap wins
        local.audit_retention_days = Some(90); // org's longer window wins
        local.egress.max_writes_per_min = Some(60); // org's tighter limit wins
        local.egress.block_secrets = Some(false); // org's true wins
        let m = merge_floor(&org, Some(&local));
        assert_eq!(m.max_context_tokens, Some(8000));
        assert_eq!(m.audit_retention_days, Some(365));
        assert_eq!(m.egress.max_writes_per_min, Some(10));
        assert_eq!(m.egress.block_secrets, Some(true));
    }

    #[test]
    fn org_pins_read_mode_over_local() {
        let mut org = rp("org");
        org.default_read_mode = Some("signatures".into());
        let mut local = rp("local");
        local.default_read_mode = Some("full".into());
        let m = merge_floor(&org, Some(&local));
        assert_eq!(m.default_read_mode.as_deref(), Some("signatures"));
    }
}
