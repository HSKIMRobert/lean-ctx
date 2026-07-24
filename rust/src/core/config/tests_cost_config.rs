use super::*;

#[test]
fn default_is_empty() {
    let cfg = CostConfig::default();
    assert!(cfg.default_model.is_none());
    assert!(cfg.models.is_empty());
    assert_eq!(cfg.model_for_client("cursor"), None);
}

#[test]
fn per_client_overrides_default() {
    let mut models = std::collections::HashMap::new();
    models.insert("cursor".to_string(), "claude-opus-4.5".to_string());
    let cfg = CostConfig {
        default_model: Some("gpt-5.4".to_string()),
        models,
        ..Default::default()
    };
    assert_eq!(
        cfg.model_for_client("cursor").as_deref(),
        Some("claude-opus-4.5")
    );
    // No entry → global default.
    assert_eq!(cfg.model_for_client("copilot").as_deref(), Some("gpt-5.4"));
}

#[test]
fn blank_values_are_ignored() {
    let cfg = CostConfig {
        default_model: Some("   ".to_string()),
        models: std::collections::HashMap::new(),
        ..Default::default()
    };
    assert_eq!(cfg.model_for_client("cursor"), None);
}

#[test]
fn parses_from_toml_section() {
    let cfg: Config = toml::from_str(
        r#"
[cost]
default_model = "claude-opus-4.5"

[cost.models]
cursor = "claude-opus-4.5"
copilot = "gpt-5.4"
"#,
    )
    .unwrap();
    assert_eq!(cfg.cost.default_model.as_deref(), Some("claude-opus-4.5"));
    assert_eq!(
        cfg.cost.model_for_client("copilot").as_deref(),
        Some("gpt-5.4")
    );
}

#[test]
fn default_config_has_empty_cost_section() {
    let cfg = Config::default();
    assert!(cfg.cost.default_model.is_none());
    assert!(cfg.cost.models.is_empty());
    assert!(cfg.cost.prices.is_empty());
}

#[test]
fn parses_price_overrides_from_toml() {
    // #1189: negotiated enterprise rates in config.toml.
    let cfg: Config = toml::from_str(
        r#"
[cost.prices."internal-llm"]
input_per_m = 0.10
output_per_m = 0.40

[cost.prices."claude-opus-4.5"]
input_per_m = 4.0
"#,
    )
    .unwrap();
    let internal = cfg.cost.prices.get("internal-llm").expect("row parsed");
    assert_eq!(internal.input_per_m, Some(0.10));
    assert_eq!(internal.output_per_m, Some(0.40));
    assert_eq!(internal.cache_read_per_m, None);
    let opus = cfg.cost.prices.get("claude-opus-4.5").expect("row parsed");
    assert_eq!(opus.input_per_m, Some(4.0));
    assert_eq!(opus.output_per_m, None);
}
