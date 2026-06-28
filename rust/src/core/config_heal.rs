//! Heal a `config.toml` that got stranded in the DATA dir (#594).
//!
//! When an older lean-ctx baked `LEAN_CTX_DATA_DIR` into an editor's MCP server
//! `env`, that process ran in single-dir mode and wrote `config.toml` into the
//! data dir (`$XDG_DATA_HOME/lean-ctx`), while the terminal CLI kept config in
//! `$XDG_CONFIG_HOME/lean-ctx`. The resolver now keeps both on the config dir
//! (see `core::paths::single_dir_override`), but a `config.toml` that was already
//! written into the data dir would be silently ignored from then on.
//!
//! This module relocates it to the canonical config dir, **losslessly**:
//! - canonical config absent/empty → the stray copy is *adopted* as the real
//!   config, so the user's settings survive the switch;
//! - canonical config already present → the CLI-authored config wins and the
//!   stray copy is moved aside to `config.toml.superseded` (never deleted).
//!
//! Idempotent and safe: it only ever touches the user's own config/data dirs,
//! moves with an atomic `rename` (copy+remove fallback across filesystems), and
//! becomes a no-op once the stray file is gone.

use std::path::{Path, PathBuf};

/// What the heal pass did with the stray data-dir `config.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealAction {
    /// The stray copy became the canonical config (canonical was absent/empty).
    Adopted,
    /// Canonical config already existed; the stray copy was moved aside.
    Superseded,
}

/// Outcome of a config-heal pass, surfaced through setup / `doctor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigHealReport {
    pub action: HealAction,
    /// The stray `config.toml` that was relocated out of the data dir.
    pub from: PathBuf,
    /// Where it landed: the canonical config (adopted) or the `.superseded` copy.
    pub to: PathBuf,
}

/// Relocate a stray data-dir `config.toml` into the canonical config dir.
/// Returns `None` when there is nothing to do (single-dir layout, or no stray
/// config in the data dir).
pub fn heal() -> Option<ConfigHealReport> {
    let config_dir = crate::core::paths::config_dir().ok()?;
    let data_dir = crate::core::paths::data_dir().ok()?;
    heal_between(&config_dir, &data_dir)
}

/// Read-only check used by `doctor`: returns the stray data-dir `config.toml`
/// that [`heal`] would relocate, or `None` when the CLI and the MCP server
/// already resolve the same config (no divergence).
pub fn pending() -> Option<PathBuf> {
    let config_dir = crate::core::paths::config_dir().ok()?;
    let data_dir = crate::core::paths::data_dir().ok()?;
    if config_dir == data_dir {
        return None;
    }
    let stray = data_dir.join("config.toml");
    file_has_content(&stray).then_some(stray)
}

/// Pure core of [`heal`], parameterized for hermetic tests.
fn heal_between(config_dir: &Path, data_dir: &Path) -> Option<ConfigHealReport> {
    // Single-dir layout (legacy/mixed/explicit pin): config legitimately lives
    // in that one directory and is not stranded.
    if config_dir == data_dir {
        return None;
    }

    let stray = data_dir.join("config.toml");
    if !file_has_content(&stray) {
        return None;
    }

    std::fs::create_dir_all(config_dir).ok()?;
    crate::core::data_dir::ensure_dir_permissions(config_dir);
    let canonical = config_dir.join("config.toml");

    if file_has_content(&canonical) {
        // The CLI-authored config is the source of truth; preserve the stray
        // copy next to it (lossless) instead of dropping it.
        let aside = data_dir.join("config.toml.superseded");
        move_overwrite(&stray, &aside).ok()?;
        return Some(ConfigHealReport {
            action: HealAction::Superseded,
            from: stray,
            to: aside,
        });
    }

    // Canonical config is absent/empty → adopt the stray copy so the user's
    // settings keep working after config resolves to the config dir.
    move_overwrite(&stray, &canonical).ok()?;
    Some(ConfigHealReport {
        action: HealAction::Adopted,
        from: stray,
        to: canonical,
    })
}

/// True when `p` is a readable file with non-whitespace content.
fn file_has_content(p: &Path) -> bool {
    std::fs::read_to_string(p).is_ok_and(|s| !s.trim().is_empty())
}

/// Move `from` onto `to`, replacing any existing file. Atomic `rename` first,
/// with a copy+remove fallback across filesystems; the source is removed only
/// after a successful copy, so an interrupted move never loses data.
fn move_overwrite(from: &Path, to: &Path) -> std::io::Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    std::fs::remove_file(from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn adopts_stray_when_canonical_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join("config");
        let data_dir = tmp.path().join("data");
        write(&data_dir.join("config.toml"), "path_jail = false\n");

        let report = heal_between(&config_dir, &data_dir).expect("should adopt");

        assert_eq!(report.action, HealAction::Adopted);
        assert_eq!(
            std::fs::read_to_string(config_dir.join("config.toml")).unwrap(),
            "path_jail = false\n"
        );
        assert!(
            !data_dir.join("config.toml").exists(),
            "stray copy must be relocated out of the data dir"
        );
    }

    #[test]
    fn supersedes_stray_when_canonical_present() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join("config");
        let data_dir = tmp.path().join("data");
        write(&config_dir.join("config.toml"), "ultra_compact = true\n");
        write(&data_dir.join("config.toml"), "STALE\n");

        let report = heal_between(&config_dir, &data_dir).expect("should supersede");

        assert_eq!(report.action, HealAction::Superseded);
        // The CLI-authored canonical config is untouched.
        assert_eq!(
            std::fs::read_to_string(config_dir.join("config.toml")).unwrap(),
            "ultra_compact = true\n"
        );
        // The stray copy is preserved aside (lossless), not deleted.
        assert!(!data_dir.join("config.toml").exists());
        assert_eq!(
            std::fs::read_to_string(data_dir.join("config.toml.superseded")).unwrap(),
            "STALE\n"
        );
    }

    #[test]
    fn noop_when_no_stray_config() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join("config");
        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        assert_eq!(heal_between(&config_dir, &data_dir), None);
    }

    #[test]
    fn noop_for_single_dir_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("lean-ctx");
        write(&dir.join("config.toml"), "x = 1\n");
        // Same dir for config and data → nothing is stranded.
        assert_eq!(heal_between(&dir, &dir), None);
    }

    #[test]
    fn is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join("config");
        let data_dir = tmp.path().join("data");
        write(&data_dir.join("config.toml"), "k = 1\n");

        assert!(heal_between(&config_dir, &data_dir).is_some());
        // Second run: stray is gone → no-op.
        assert_eq!(heal_between(&config_dir, &data_dir), None);
    }
}
