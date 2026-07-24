use super::*;

#[test]
fn default_is_off() {
    // Guard against a stray env var leaking into the test process.
    if std::env::var("LEAN_CTX_PERMISSION_INHERITANCE").is_ok() {
        return;
    }
    let cfg = Config::default();
    assert_eq!(
        cfg.permission_inheritance_effective(),
        PermissionInheritance::Off
    );
}

#[test]
fn config_on() {
    if std::env::var("LEAN_CTX_PERMISSION_INHERITANCE").is_ok() {
        return;
    }
    let cfg = Config {
        permission_inheritance: Some("on".to_string()),
        ..Default::default()
    };
    assert_eq!(
        cfg.permission_inheritance_effective(),
        PermissionInheritance::On
    );
}

#[test]
fn unknown_value_falls_back_to_off() {
    if std::env::var("LEAN_CTX_PERMISSION_INHERITANCE").is_ok() {
        return;
    }
    let cfg = Config {
        permission_inheritance: Some("nonsense".to_string()),
        ..Default::default()
    };
    assert_eq!(
        cfg.permission_inheritance_effective(),
        PermissionInheritance::Off
    );
}

#[test]
fn deserialization_from_toml() {
    let cfg: Config = toml::from_str(r#"permission_inheritance = "on""#).unwrap();
    assert_eq!(cfg.permission_inheritance.as_deref(), Some("on"));
}

#[test]
fn local_override_merges() {
    if std::env::var("LEAN_CTX_PERMISSION_INHERITANCE").is_ok() {
        return;
    }
    let mut base = Config::default();
    base.merge_local(r#"permission_inheritance = "on""#, true);
    assert_eq!(
        base.permission_inheritance_effective(),
        PermissionInheritance::On
    );
}
