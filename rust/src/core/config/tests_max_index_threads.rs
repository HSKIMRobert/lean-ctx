use super::*;

#[test]
fn default_is_zero() {
    assert_eq!(Config::default().max_index_threads, 0);
}

#[test]
fn absent_in_toml_defaults_to_zero() {
    let cfg: Config = toml::from_str("").unwrap();
    assert_eq!(cfg.max_index_threads, 0);
}

#[test]
fn parses_explicit_cap_from_toml() {
    let cfg: Config = toml::from_str("max_index_threads = 8").unwrap();
    assert_eq!(cfg.max_index_threads, 8);
}

#[test]
fn effective_uses_config_when_env_unset() {
    // Only meaningful when LEANCTX_INDEX_THREADS is unset; skip otherwise.
    if std::env::var("LEANCTX_INDEX_THREADS").is_ok() {
        return;
    }
    let cfg = Config {
        max_index_threads: 4,
        ..Default::default()
    };
    assert_eq!(cfg.max_index_threads_effective(), 4);
}

#[test]
fn effective_zero_means_no_cap() {
    if std::env::var("LEANCTX_INDEX_THREADS").is_ok() {
        return;
    }
    assert_eq!(Config::default().max_index_threads_effective(), 0);
}
