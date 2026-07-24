use super::*;

#[test]
fn default_is_shared() {
    let cfg = Config::default();
    assert_eq!(cfg.rules_injection_effective(), RulesInjection::Shared);
}

#[test]
fn config_dedicated() {
    let cfg = Config {
        rules_injection: Some("dedicated".to_string()),
        ..Default::default()
    };
    assert_eq!(cfg.rules_injection_effective(), RulesInjection::Dedicated);
}

#[test]
fn config_off() {
    for raw in ["off", "none", "disabled"] {
        let cfg = Config {
            rules_injection: Some(raw.to_string()),
            ..Default::default()
        };
        assert_eq!(
            cfg.rules_injection_effective(),
            RulesInjection::Off,
            "{raw:?} should resolve to Off"
        );
    }
}

#[test]
fn off_disables_dedicated_session_context() {
    let cfg = Config {
        rules_injection: Some("off".to_string()),
        ..Default::default()
    };
    assert!(!cfg.dedicated_session_context_active());
}

#[test]
fn unknown_value_falls_back_to_shared() {
    let cfg = Config {
        rules_injection: Some("nonsense".to_string()),
        ..Default::default()
    };
    assert_eq!(cfg.rules_injection_effective(), RulesInjection::Shared);
}

#[test]
fn deserialization_from_toml() {
    let cfg: Config = toml::from_str(r#"rules_injection = "dedicated""#).unwrap();
    assert_eq!(cfg.rules_injection.as_deref(), Some("dedicated"));
    assert_eq!(cfg.rules_injection_effective(), RulesInjection::Dedicated);
}

#[test]
fn dedicated_session_context_gated_by_scope() {
    // Dedicated + non-project scope → SessionStart summary active.
    let cfg = Config {
        rules_injection: Some("dedicated".to_string()),
        ..Default::default()
    };
    assert!(cfg.dedicated_session_context_active());

    // Dedicated + project scope → global summary suppressed (project files only).
    let cfg = Config {
        rules_injection: Some("dedicated".to_string()),
        rules_scope: Some("project".to_string()),
        ..Default::default()
    };
    assert!(!cfg.dedicated_session_context_active());

    // Shared (default) → never the SessionStart summary path.
    let cfg = Config::default();
    assert!(!cfg.dedicated_session_context_active());
}

#[test]
fn local_override_merges() {
    let mut base = Config::default();
    base.merge_local(r#"rules_injection = "dedicated""#, true);
    assert_eq!(base.rules_injection_effective(), RulesInjection::Dedicated);
}
