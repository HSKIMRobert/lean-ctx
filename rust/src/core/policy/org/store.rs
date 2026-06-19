//! Where the active org policy artifact lives, and how it gets there (GL #674).
//!
//! The runtime loads the installed artifact from a **pluggable source**, checked
//! in order:
//! 1. `LEANCTX_ORG_POLICY` — an explicit path (CI, containers, MDM that drops
//!    the file wherever it likes);
//! 2. `<config_dir>/org-policy.signed.json` — the location `policy org install`
//!    writes to, picked up automatically on the next run.
//!
//! This module only *locates, reads and writes* the artifact bytes — signature
//! verification and trust pinning are layered on top in [`super`], so the source
//! stays swappable without touching the security checks.

use std::path::{Path, PathBuf};

use super::model::OrgPolicyV1;

/// Env override pointing at an explicit org-policy artifact path.
const POLICY_ENV: &str = "LEANCTX_ORG_POLICY";

/// Filename of the installed artifact under the config dir.
const INSTALLED_FILE: &str = "org-policy.signed.json";

/// The canonical installed location (`<config_dir>/org-policy.signed.json`).
pub fn installed_path() -> Result<PathBuf, String> {
    Ok(crate::core::paths::config_dir()?.join(INSTALLED_FILE))
}

/// Resolve the active artifact path from the pluggable source chain, or `None`
/// when no org policy is present (the common, opt-out-by-default case).
#[must_use]
pub fn source_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var(POLICY_ENV) {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.exists() {
                return Some(path);
            }
        }
    }
    let installed = installed_path().ok()?;
    installed.exists().then_some(installed)
}

/// Read + parse the artifact at `path` (no signature/trust check).
pub fn read(path: &Path) -> Result<OrgPolicyV1, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    OrgPolicyV1::from_json(&text)
}

/// Read + parse the active artifact from the source chain (no signature/trust
/// check). `None` when no org policy is present.
#[must_use]
pub fn load_active() -> Option<OrgPolicyV1> {
    let path = source_path()?;
    match read(&path) {
        Ok(a) => Some(a),
        Err(e) => {
            tracing::warn!("org policy: ignoring unreadable {} ({e})", path.display());
            None
        }
    }
}

/// Write `artifact` to the installed location, returning that path. Callers
/// (the `install` CLI) verify signature + trust *before* calling this.
pub fn install(artifact: &OrgPolicyV1) -> Result<PathBuf, String> {
    let path = installed_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir config: {e}"))?;
    }
    std::fs::write(&path, artifact.to_json()?)
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

/// Remove the installed artifact (used by `policy org uninstall`). Returns
/// `true` when a file was removed.
pub fn uninstall() -> Result<bool, String> {
    let path = installed_path()?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("remove {}: {e}", path.display()))?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data_dir::isolated_data_dir;

    const PACK: &str = r#"
name = "acme-floor"
version = "1.0.0"
description = "ACME org floor"

[context]
deny_tools = ["ctx_url_read"]
"#;

    fn signed() -> OrgPolicyV1 {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).unwrap();
        let key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let mut a = OrgPolicyV1::build("acme", "1", true, PACK).unwrap();
        a.sign_with_key(&key);
        a
    }

    #[test]
    fn install_then_load_roundtrips() {
        let _iso = isolated_data_dir();
        assert!(source_path().is_none());
        let a = signed();
        install(&a).unwrap();
        let loaded = load_active().expect("installed artifact loads");
        assert_eq!(loaded, a);
        assert!(uninstall().unwrap());
        assert!(source_path().is_none());
    }

    #[test]
    fn env_path_takes_precedence() {
        let _iso = isolated_data_dir();
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("explicit.json");
        std::fs::write(&p, signed().to_json().unwrap()).unwrap();
        crate::test_env::set_var(POLICY_ENV, p.to_str().unwrap());
        assert_eq!(source_path().as_deref(), Some(p.as_path()));
        crate::test_env::remove_var(POLICY_ENV);
    }
}
