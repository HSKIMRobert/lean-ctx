use std::path::PathBuf;

/// Resolve the lean-ctx data directory.
///
/// Priority order (backward-compatible XDG migration):
/// 1. `LEAN_CTX_DATA_DIR` env var (explicit override)
/// 2. `~/.lean-ctx` if it already exists (don't break existing installs)
/// 3. `$XDG_CONFIG_HOME/lean-ctx` (XDG compliant, default `~/.config/lean-ctx`)
pub fn lean_ctx_data_dir() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("LEAN_CTX_DATA_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            ensure_dir_permissions(&p);
            return Ok(p);
        }
    }

    let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    let legacy = home.join(".lean-ctx");
    if legacy.exists() {
        ensure_dir_permissions(&legacy);
        return Ok(legacy);
    }

    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| home.join(".config"), PathBuf::from);

    let dir = xdg_config.join("lean-ctx");
    ensure_dir_permissions(&dir);
    Ok(dir)
}

#[cfg(unix)]
fn ensure_dir_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if path.is_dir() {
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
    }
}

#[cfg(not(unix))]
fn ensure_dir_permissions(_path: &std::path::Path) {}

pub fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let mutex = LOCK.get_or_init(|| Mutex::new(()));
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
