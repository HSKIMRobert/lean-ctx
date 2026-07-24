use super::*;

#[test]
fn defaults_are_reasonable() {
    let cfg = LoopDetectionConfig::default();
    assert_eq!(cfg.normal_threshold, 2);
    assert_eq!(cfg.reduced_threshold, 4);
    // 0 = blocking disabled by default (LeanCTX philosophy: always help, never block)
    assert_eq!(cfg.blocked_threshold, 0);
    assert_eq!(cfg.window_secs, 300);
    assert_eq!(cfg.search_group_limit, 10);
}

#[test]
fn deserialization_defaults_when_missing() {
    let cfg: Config = toml::from_str("").unwrap();
    // 0 = blocking disabled by default
    assert_eq!(cfg.loop_detection.blocked_threshold, 0);
    assert_eq!(cfg.loop_detection.search_group_limit, 10);
}

#[test]
fn deserialization_from_toml() {
    let cfg: Config = toml::from_str(
        r"
        [loop_detection]
        normal_threshold = 1
        reduced_threshold = 3
        blocked_threshold = 5
        window_secs = 120
        search_group_limit = 8
        ",
    )
    .unwrap();
    assert_eq!(cfg.loop_detection.normal_threshold, 1);
    assert_eq!(cfg.loop_detection.reduced_threshold, 3);
    assert_eq!(cfg.loop_detection.blocked_threshold, 5);
    assert_eq!(cfg.loop_detection.window_secs, 120);
    assert_eq!(cfg.loop_detection.search_group_limit, 8);
}

#[test]
fn partial_override_keeps_defaults() {
    let cfg: Config = toml::from_str(
        r"
        [loop_detection]
        blocked_threshold = 10
        ",
    )
    .unwrap();
    assert_eq!(cfg.loop_detection.blocked_threshold, 10);
    assert_eq!(cfg.loop_detection.normal_threshold, 2);
    assert_eq!(cfg.loop_detection.search_group_limit, 10);
}
