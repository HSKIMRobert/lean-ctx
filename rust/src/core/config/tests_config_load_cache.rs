use super::*;
use crate::core::data_dir::isolated_data_dir;

/// #406 regression: `Config::load()` must reflect a content change even when
/// the file mtime is unchanged. A mtime-only cache (the old behaviour) kept a
/// long-lived MCP server on a stale `path_jail` while a fresh `doctor`
/// process — with an empty cache — saw the new value. The cache is now keyed
/// on a content hash, so this scenario stays live.
#[test]
fn load_honors_content_change_with_preserved_mtime() {
    let _iso = isolated_data_dir();
    let path = Config::path().expect("config path under isolated data dir");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    // Warm the cache with jail unset (default None).
    std::fs::write(&path, "# initial\n").unwrap();
    let mtime0 = std::fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(Config::load().path_jail, None);

    // Flip path_jail=false but restore the original mtime, so any mtime-only
    // cache would serve the stale value (#406).
    std::fs::write(&path, "path_jail = false\n").unwrap();
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(mtime0)).unwrap();

    assert_eq!(
        Config::load().path_jail,
        Some(false),
        "Config::load() must honor a content change with unchanged mtime (#406)"
    );
}
