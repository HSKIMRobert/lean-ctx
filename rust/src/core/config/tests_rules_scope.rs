use super::*;

#[test]
fn default_is_both() {
    let cfg = Config::default();
    assert_eq!(cfg.rules_scope_effective(), RulesScope::Both);
}

#[test]
fn config_global() {
    let cfg = Config {
        rules_scope: Some("global".to_string()),
        ..Default::default()
    };
    assert_eq!(cfg.rules_scope_effective(), RulesScope::Global);
}

#[test]
fn config_project() {
    let cfg = Config {
        rules_scope: Some("project".to_string()),
        ..Default::default()
    };
    assert_eq!(cfg.rules_scope_effective(), RulesScope::Project);
}

#[test]
fn unknown_value_falls_back_to_both() {
    let cfg = Config {
        rules_scope: Some("nonsense".to_string()),
        ..Default::default()
    };
    assert_eq!(cfg.rules_scope_effective(), RulesScope::Both);
}

#[test]
fn deserialization_none_by_default() {
    let cfg: Config = toml::from_str("").unwrap();
    assert!(cfg.rules_scope.is_none());
    assert_eq!(cfg.rules_scope_effective(), RulesScope::Both);
}

#[test]
fn deserialization_from_toml() {
    let cfg: Config = toml::from_str(r#"rules_scope = "project""#).unwrap();
    assert_eq!(cfg.rules_scope.as_deref(), Some("project"));
    assert_eq!(cfg.rules_scope_effective(), RulesScope::Project);
}
