use super::*;

#[test]
fn default_budget_is_eight_thousand() {
    assert_eq!(ContextConfig::default().budget_tokens, 8000);
    assert_eq!(Config::default().context.budget_tokens, 8000);
    assert!(Config::default().context.proactive_expansion);
    assert_eq!(
        Config::default().context.proactive_expansion_budget_tokens,
        2000
    );
    assert_eq!(
        Config::default().context.proactive_expansion_max_age_secs,
        3600
    );
}

#[test]
fn effective_uses_config_field_when_no_env() {
    let _lock = crate::core::data_dir::test_env_lock();
    if std::env::var("LEAN_CTX_CONTEXT_BUDGET_TOKENS").is_ok() {
        return;
    }
    let cfg = Config {
        context: ContextConfig {
            budget_tokens: 5000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(cfg.context_budget_tokens_effective(), 5000);
}

#[test]
fn env_overrides_config_budget() {
    let _lock = crate::core::data_dir::test_env_lock();
    crate::test_env::set_var("LEAN_CTX_CONTEXT_BUDGET_TOKENS", "1234");
    let cfg = Config {
        context: ContextConfig {
            budget_tokens: 5000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(cfg.context_budget_tokens_effective(), 1234);
    crate::test_env::remove_var("LEAN_CTX_CONTEXT_BUDGET_TOKENS");
}

#[test]
fn proactive_expansion_settings_are_effective() {
    let cfg = Config {
        context: ContextConfig {
            proactive_expansion: false,
            proactive_expansion_budget_tokens: 17,
            proactive_expansion_threshold: 0.8,
            proactive_expansion_max_age_secs: 42,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(!cfg.proactive_expansion_effective());
    assert_eq!(cfg.proactive_expansion_budget_tokens_effective(), 17);
    assert_eq!(cfg.proactive_expansion_threshold_effective(), 0.8);
    assert_eq!(cfg.proactive_expansion_max_age_secs_effective(), 42);
}
