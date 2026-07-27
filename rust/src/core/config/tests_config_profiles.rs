use super::*;

const PROFILED_CONFIG: &str = r#"
theme = "base"
max_ram_percent = 20
config_profile = "local"

[archive]
max_disk_mb = 100

[profiles.local]
theme = "local"

[profiles.local.archive]
max_disk_mb = 25

[profiles.cloud]
theme = "cloud"
"#;

#[test]
fn configured_profile_overrides_base_recursively() {
    let cfg = super::super::loader::parse_config_with_profile(PROFILED_CONFIG, None).unwrap();

    assert_eq!(cfg.theme, "local");
    assert_eq!(cfg.max_ram_percent, 20);
    assert_eq!(cfg.archive.max_disk_mb, 25);
    assert_eq!(cfg.config_profile.as_deref(), Some("local"));
    assert_eq!(cfg.profiles.len(), 2);
}

#[test]
fn explicit_profile_takes_precedence_over_persisted_selector() {
    let cfg =
        super::super::loader::parse_config_with_profile(PROFILED_CONFIG, Some("cloud")).unwrap();

    assert_eq!(cfg.theme, "cloud");
    assert_eq!(cfg.archive.max_disk_mb, 100);
    assert_eq!(cfg.config_profile.as_deref(), Some("local"));
}

#[test]
fn selected_profile_must_exist() {
    let error = super::super::loader::parse_config_with_profile(PROFILED_CONFIG, Some("missing"))
        .unwrap_err();

    assert_eq!(error, "config profile 'missing' is not defined");
}

#[test]
fn profile_cannot_change_profile_selection() {
    let error = super::super::loader::parse_config_with_profile(
        r#"
config_profile = "local"

[profiles.local]
config_profile = "cloud"
"#,
        None,
    )
    .unwrap_err();

    assert_eq!(
        error,
        "config profile 'local' cannot override reserved profile keys"
    );
}

#[test]
fn project_local_config_applies_its_selected_profile() {
    let mut cfg = Config::default();

    cfg.merge_local(PROFILED_CONFIG, true);

    assert_eq!(cfg.theme, "local");
    assert_eq!(cfg.archive.max_disk_mb, 25);
}

#[test]
fn project_local_config_inherits_the_global_selector() {
    let mut cfg = Config {
        config_profile: Some("cloud".to_string()),
        ..Config::default()
    };

    cfg.merge_local(
        r#"
[profiles.local]
theme = "local"

[profiles.cloud]
theme = "cloud"
"#,
        true,
    );

    assert_eq!(cfg.theme, "cloud");
}
