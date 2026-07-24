use super::*;

// #540: `missing_config_path()` is the visible signal that the runtime
// resolved a `config.toml` that doesn't exist — i.e. an edit landed in a
// different file (XDG vs legacy dir, or a sandboxed HOME). The block messages
// turn this into an over-MCP-visible note.
#[test]
fn missing_config_path_flags_absent_then_clears_when_present() {
    let _lock = crate::core::data_dir::test_env_lock();
    let saved = std::env::var("LEAN_CTX_CONFIG_DIR").ok();
    let dir = tempfile::tempdir().unwrap();
    crate::test_env::set_var("LEAN_CTX_CONFIG_DIR", dir.path());

    // No config.toml in the resolved dir → flagged as missing (on defaults).
    assert_eq!(
        Config::missing_config_path(),
        Some(dir.path().join("config.toml"))
    );

    // Create the file → the runtime now has a real config, so no flag.
    std::fs::write(dir.path().join("config.toml"), "ultra_compact = true\n").unwrap();
    assert!(Config::missing_config_path().is_none());

    crate::test_env::remove_var("LEAN_CTX_CONFIG_DIR");
    if let Some(v) = saved {
        crate::test_env::set_var("LEAN_CTX_CONFIG_DIR", v);
    }
}
