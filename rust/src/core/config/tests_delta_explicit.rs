use super::*;

// --- Defaults ---

#[test]
fn default_is_false() {
    let cfg = Config::default();
    assert!(!cfg.delta_explicit);
}

#[test]
fn effective_false_when_unset() {
    // Serialize with the env-mutating sibling test below: this reads the var
    // as a guard and again inside delta_explicit_effective(), so without the
    // shared lock a parallel set_var between the two reads flips the result.
    let _lock = crate::core::data_dir::test_env_lock();
    if std::env::var("LCTX_DELTA_EXPLICIT").is_ok() {
        return;
    }
    let cfg = Config::default();
    assert!(!cfg.delta_explicit_effective());
}

// --- Config field ---

#[test]
fn config_field_true_respected_when_no_env() {
    // Serialize with the env-mutating sibling test (see effective_false_when_unset).
    let _lock = crate::core::data_dir::test_env_lock();
    if std::env::var("LCTX_DELTA_EXPLICIT").is_ok() {
        return;
    }
    let cfg = Config {
        delta_explicit: true,
        ..Default::default()
    };
    assert!(cfg.delta_explicit_effective());
}

#[test]
fn config_field_false_respected_when_no_env() {
    // Serialize with the env-mutating sibling test (see effective_false_when_unset).
    let _lock = crate::core::data_dir::test_env_lock();
    if std::env::var("LCTX_DELTA_EXPLICIT").is_ok() {
        return;
    }
    let cfg = Config {
        delta_explicit: false,
        ..Default::default()
    };
    assert!(!cfg.delta_explicit_effective());
}

// --- Env override (both directions) ---

#[test]
fn env_overrides_config_field_in_both_directions() {
    // All env mutation serializes through this lock (Rust 2024 set_var is
    // `unsafe`; the lock is the documented soundness precondition).
    let _lock = crate::core::data_dir::test_env_lock();

    // env=1 turns the feature ON even when the config field is false.
    crate::test_env::set_var("LCTX_DELTA_EXPLICIT", "1");
    let off_cfg = Config {
        delta_explicit: false,
        ..Default::default()
    };
    assert!(
        off_cfg.delta_explicit_effective(),
        "LCTX_DELTA_EXPLICIT=1 must enable the feature over a false config field"
    );

    // env=0 forces it OFF even when the config field is true.
    crate::test_env::set_var("LCTX_DELTA_EXPLICIT", "0");
    let on_cfg = Config {
        delta_explicit: true,
        ..Default::default()
    };
    assert!(
        !on_cfg.delta_explicit_effective(),
        "LCTX_DELTA_EXPLICIT=0 must disable the feature over a true config field"
    );

    // `true`/`false` spellings are honoured too (case-insensitive).
    crate::test_env::set_var("LCTX_DELTA_EXPLICIT", "true");
    assert!(off_cfg.delta_explicit_effective());
    crate::test_env::set_var("LCTX_DELTA_EXPLICIT", "FALSE");
    assert!(!on_cfg.delta_explicit_effective());

    // Restore: with the var removed the config field decides again.
    crate::test_env::remove_var("LCTX_DELTA_EXPLICIT");
    assert!(on_cfg.delta_explicit_effective());
    assert!(!off_cfg.delta_explicit_effective());
}

// --- TOML deserialization ---

#[test]
fn deserialization_true() {
    let cfg: Config = toml::from_str("delta_explicit = true").unwrap();
    assert!(cfg.delta_explicit);
}

#[test]
fn deserialization_false() {
    let cfg: Config = toml::from_str("delta_explicit = false").unwrap();
    assert!(!cfg.delta_explicit);
}

#[test]
fn deserialization_absent_defaults_false() {
    let cfg: Config = toml::from_str("").unwrap();
    assert!(!cfg.delta_explicit);
}

// --- Round-trip (serialize → deserialize preserves the field) ---

#[test]
fn round_trip_preserves_field() {
    let cfg = Config {
        delta_explicit: true,
        ..Default::default()
    };
    let serialized = toml::to_string(&cfg).expect("Config must serialize to TOML");
    let restored: Config = toml::from_str(&serialized).expect("serialized Config must round-trip");
    assert!(
        restored.delta_explicit,
        "delta_explicit must survive a TOML serialize → deserialize round-trip"
    );
}

// --- Coexistence with other config fields ---

#[test]
fn delta_explicit_independent_of_no_degrade() {
    // Serialize with the env-mutating sibling tests (see effective_false_when_unset).
    let _lock = crate::core::data_dir::test_env_lock();
    if std::env::var("LCTX_DELTA_EXPLICIT").is_ok() || std::env::var("LCTX_NO_DEGRADE").is_ok() {
        return;
    }
    let cfg = Config {
        delta_explicit: true,
        no_degrade: true,
        ..Default::default()
    };
    assert!(cfg.delta_explicit_effective());
    assert!(cfg.no_degrade_effective());
}
