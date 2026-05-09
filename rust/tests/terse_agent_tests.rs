use lean_ctx::core::config::{CompressionLevel, Config, TerseAgent};
use lean_ctx::instructions;
use lean_ctx::tools::CrpMode;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn set_env(compression: &str, terse: &str) {
    std::env::set_var("LEAN_CTX_COMPRESSION", compression);
    std::env::set_var("LEAN_CTX_TERSE_AGENT", terse);
}

fn cleanup_env() {
    std::env::remove_var("LEAN_CTX_COMPRESSION");
    std::env::remove_var("LEAN_CTX_TERSE_AGENT");
}

// ── TerseAgent unit tests (no Config::load dependency) ──

#[test]
fn terse_agent_default_is_off() {
    let ta = TerseAgent::default();
    assert!(!ta.is_active());
    assert!(matches!(ta, TerseAgent::Off));
}

#[test]
fn terse_agent_from_env() {
    let _g = lock();
    std::env::set_var("LEAN_CTX_TERSE_AGENT", "full");
    let ta = TerseAgent::from_env();
    assert!(matches!(ta, TerseAgent::Full));
    assert!(ta.is_active());

    std::env::set_var("LEAN_CTX_TERSE_AGENT", "lite");
    assert!(matches!(TerseAgent::from_env(), TerseAgent::Lite));

    std::env::set_var("LEAN_CTX_TERSE_AGENT", "ultra");
    assert!(matches!(TerseAgent::from_env(), TerseAgent::Ultra));

    std::env::set_var("LEAN_CTX_TERSE_AGENT", "off");
    assert!(matches!(TerseAgent::from_env(), TerseAgent::Off));

    std::env::set_var("LEAN_CTX_TERSE_AGENT", "0");
    assert!(matches!(TerseAgent::from_env(), TerseAgent::Off));

    cleanup_env();
}

#[test]
fn terse_agent_effective_env_overrides_config() {
    let _g = lock();
    std::env::set_var("LEAN_CTX_TERSE_AGENT", "ultra");
    let effective = TerseAgent::effective(&TerseAgent::Off);
    assert!(matches!(effective, TerseAgent::Ultra));
    cleanup_env();
}

#[test]
fn terse_agent_effective_falls_back_to_config() {
    let _g = lock();
    std::env::remove_var("LEAN_CTX_TERSE_AGENT");
    let effective = TerseAgent::effective(&TerseAgent::Full);
    assert!(matches!(effective, TerseAgent::Full));
    cleanup_env();
}

// ── Legacy TerseAgent instruction injection (CompressionLevel forced off) ──

#[test]
fn legacy_terse_lite_injects_output_style() {
    let _g = lock();
    set_env("off", "lite");
    let text = instructions::build_instructions(CrpMode::Off);
    assert!(
        text.contains("OUTPUT STYLE"),
        "legacy terse lite should inject OUTPUT STYLE block"
    );
    assert!(
        text.contains("concise"),
        "legacy terse lite should mention concise"
    );
    cleanup_env();
}

#[test]
fn legacy_terse_full_injects_density() {
    let _g = lock();
    set_env("off", "full");
    let text = instructions::build_instructions(CrpMode::Off);
    assert!(
        text.contains("Maximum density"),
        "legacy terse full should mention max density"
    );
    cleanup_env();
}

#[test]
fn legacy_terse_ultra_injects_expert_mode() {
    let _g = lock();
    set_env("off", "ultra");
    let text = instructions::build_instructions(CrpMode::Off);
    assert!(
        text.contains("Ultra-terse"),
        "legacy terse ultra should contain ultra-terse"
    );
    assert!(
        text.contains("pair programmer"),
        "legacy terse ultra should mention pair programmer"
    );
    cleanup_env();
}

#[test]
fn legacy_terse_off_no_output_style() {
    let _g = lock();
    set_env("off", "off");
    let text = instructions::build_instructions(CrpMode::Off);
    assert!(
        !text.contains("OUTPUT STYLE"),
        "all off should not inject OUTPUT STYLE block"
    );
    cleanup_env();
}

#[test]
fn legacy_terse_lite_with_tdd_skipped() {
    let _g = lock();
    set_env("off", "lite");
    let text = instructions::build_instructions(CrpMode::Tdd);
    assert!(
        !text.contains("OUTPUT STYLE"),
        "legacy terse lite should be skipped when CRP=tdd (already dense enough)"
    );
    cleanup_env();
}

#[test]
fn legacy_terse_ultra_with_tdd_still_active() {
    let _g = lock();
    set_env("off", "ultra");
    let text = instructions::build_instructions(CrpMode::Tdd);
    assert!(
        text.contains("Ultra-terse"),
        "legacy terse ultra should still apply on top of CRP=tdd"
    );
    cleanup_env();
}

// ── New CompressionLevel instruction injection ──

#[test]
fn compression_level_lite_injects_concise() {
    let _g = lock();
    set_env("lite", "off");
    let text = instructions::build_instructions(CrpMode::Off);
    assert!(
        text.contains("OUTPUT STYLE: concise"),
        "compression lite should inject concise prompt"
    );
    cleanup_env();
}

#[test]
fn compression_level_standard_injects_dense() {
    let _g = lock();
    set_env("standard", "off");
    let text = instructions::build_instructions(CrpMode::Off);
    assert!(
        text.contains("OUTPUT STYLE: dense"),
        "compression standard should inject dense prompt"
    );
    assert!(
        text.contains("fn, cfg, impl"),
        "compression standard should mention abbreviations"
    );
    cleanup_env();
}

#[test]
fn compression_level_max_injects_expert_terse() {
    let _g = lock();
    set_env("max", "off");
    let text = instructions::build_instructions(CrpMode::Off);
    assert!(
        text.contains("OUTPUT STYLE: expert-terse"),
        "compression max should inject expert-terse prompt"
    );
    assert!(
        text.contains("Telegraph"),
        "compression max should mention telegraph format"
    );
    cleanup_env();
}

#[test]
fn compression_level_off_no_output_style() {
    let _g = lock();
    set_env("off", "off");
    let text = instructions::build_instructions(CrpMode::Off);
    assert!(
        !text.contains("OUTPUT STYLE"),
        "both off should not inject any OUTPUT STYLE block"
    );
    cleanup_env();
}

#[test]
fn compression_env_overrides_legacy_terse_agent() {
    let _g = lock();
    set_env("max", "lite");
    let text = instructions::build_instructions(CrpMode::Off);
    assert!(
        text.contains("expert-terse"),
        "compression env should override legacy terse_agent"
    );
    assert!(
        !text.contains("Ultra-terse"),
        "legacy ultra text should not appear when compression env is set"
    );
    cleanup_env();
}

// ── Config deserialization ──

#[test]
fn terse_agent_config_deserializes() {
    let toml = r#"
terse_agent = "full"
"#;
    let config: Config = toml::from_str(toml).expect("should parse terse_agent from toml");
    assert!(matches!(config.terse_agent, TerseAgent::Full));
}

#[test]
fn terse_agent_config_default_off() {
    let toml = "";
    let config: Config = toml::from_str(toml).expect("empty toml should use defaults");
    assert!(matches!(config.terse_agent, TerseAgent::Off));
}

#[test]
fn compression_level_config_deserializes() {
    let toml = r#"compression_level = "standard""#;
    let config: Config = toml::from_str(toml).expect("should parse compression_level");
    assert!(matches!(
        config.compression_level,
        CompressionLevel::Standard
    ));
}

#[test]
fn compression_level_config_default_off() {
    let toml = "";
    let config: Config = toml::from_str(toml).expect("empty toml should use defaults");
    assert!(matches!(config.compression_level, CompressionLevel::Off));
}
