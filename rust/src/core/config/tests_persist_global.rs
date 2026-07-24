use super::*;

// Regression tests for #443: persisting config must never reset customized
// values nor leak project-local overrides into the global file.

fn tmp_config() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    (dir, path)
}

// The canonical persist path keeps every customized value and applies only
// the requested change.
#[test]
fn update_global_at_preserves_customized_and_persists_change() {
    let (_dir, path) = tmp_config();
    std::fs::write(
        &path,
        "max_ram_percent = 30\ncompression_level = \"standard\"\n",
    )
    .unwrap();

    let returned = Config::update_global_at(&path, |c| c.proxy_enabled = Some(true))
        .expect("update_global_at must succeed");
    assert_eq!(returned.proxy_enabled, Some(true));

    let reloaded: Config = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        reloaded.max_ram_percent, 30,
        "customized value must survive"
    );
    assert_eq!(reloaded.compression_level, CompressionLevel::Standard);
    assert_eq!(reloaded.proxy_enabled, Some(true));
}

// load_global never folds in project-local overrides; it reads only the
// global file. update_global builds on this, so persists cannot leak.
#[test]
fn load_global_from_reads_only_the_given_file() {
    let (_dir, path) = tmp_config();
    std::fs::write(&path, "theme = \"global-theme\"\n").unwrap();
    let cfg = Config::load_global_from(&path);
    assert_eq!(cfg.theme, "global-theme");
}

// Root-cause marker: the OLD `load() (with merge_local) -> save()` pattern
// leaks a project-local override into the global file. This proves why
// persist paths must use load_global / update_global instead.
#[test]
fn merged_load_then_save_leaks_local_override_root_cause_marker() {
    let (_dir, path) = tmp_config();
    std::fs::write(&path, "theme = \"global-theme\"\n").unwrap();

    // Simulate `Config::load()`: global file + project-local override merged.
    let mut cfg = Config::load_global_from(&path);
    cfg.merge_local("theme = \"project-local\"\n", true);
    // OLD persist: write the merged struct back to the GLOBAL file.
    cfg.save_to(&path).unwrap();

    let reloaded: Config = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        reloaded.theme, "project-local",
        "OLD load+merge_local+save leaks the project-local value into global (#443)"
    );
}

// Subticket 4 contract: refuse to touch an unparseable config; never clobber.
#[test]
fn update_global_at_refuses_unparseable_and_leaves_file_untouched() {
    let (_dir, path) = tmp_config();
    let corrupt = "max_ram_percent = = =\n";
    std::fs::write(&path, corrupt).unwrap();

    let result = Config::update_global_at(&path, |c| c.proxy_enabled = Some(true));
    assert!(
        result.is_err(),
        "must refuse to modify an unparseable config"
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        corrupt,
        "the corrupt file must be left exactly as-is"
    );
}

#[test]
fn load_global_from_missing_or_empty_yields_defaults() {
    let (_dir, path) = tmp_config();
    // Missing file.
    let cfg = Config::load_global_from(&path);
    assert_eq!(cfg.max_ram_percent, Config::default().max_ram_percent);
    // Empty / whitespace-only file.
    std::fs::write(&path, "   \n").unwrap();
    let cfg2 = Config::load_global_from(&path);
    assert_eq!(cfg2.max_ram_percent, Config::default().max_ram_percent);
}
