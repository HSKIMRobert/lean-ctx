use std::path::{Path, PathBuf};

const IDE_CONFIG_DIRS: &[&str] = &[
    ".lean-ctx",
    ".cursor",
    ".claude",
    ".codex",
    ".codeium",
    ".gemini",
    ".qwen",
    ".trae",
    ".kiro",
    ".verdent",
    ".pi",
    ".amp",
    ".aider",
    ".continue",
];

pub fn allow_paths_from_env_and_config() -> Vec<PathBuf> {
    let mut out = Vec::new();

    if let Ok(data_dir) = crate::core::data_dir::lean_ctx_data_dir() {
        out.push(canonicalize_or_self(&data_dir));
    }

    if let Some(home) = dirs::home_dir() {
        for dir in IDE_CONFIG_DIRS {
            let p = home.join(dir);
            if p.exists() {
                out.push(canonicalize_or_self(&p));
            }
        }
    }

    let cfg = crate::core::config::Config::load();
    for p in &cfg.allow_paths {
        let pb = PathBuf::from(p);
        out.push(canonicalize_or_self(&pb));
    }
    for p in &cfg.extra_roots {
        let pb = PathBuf::from(p);
        out.push(canonicalize_or_self(&pb));
    }

    let v = std::env::var("LCTX_ALLOW_PATH")
        .or_else(|_| std::env::var("LEAN_CTX_ALLOW_PATH"))
        .unwrap_or_default();
    if !v.trim().is_empty() {
        for p in std::env::split_paths(&v) {
            out.push(canonicalize_or_self(&p));
        }
    }

    let extra = std::env::var("LEAN_CTX_EXTRA_ROOTS").unwrap_or_default();
    if !extra.trim().is_empty() {
        for p in std::env::split_paths(&extra) {
            out.push(canonicalize_or_self(&p));
        }
    }

    out
}

fn is_under_prefix(path: &Path, prefix: &Path) -> bool {
    path.starts_with(prefix)
}

pub fn canonicalize_or_self(path: &Path) -> PathBuf {
    super::pathutil::safe_canonicalize_bounded(path, 2000)
}

fn canonicalize_existing_ancestor(path: &Path) -> Option<(PathBuf, Vec<std::ffi::OsString>)> {
    let mut cur = path.to_path_buf();
    let mut remainder: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if cur.exists() {
            return Some((canonicalize_or_self(&cur), remainder));
        }
        let name = cur.file_name()?.to_os_string();
        remainder.push(name);
        if !cur.pop() {
            return None;
        }
    }
}

pub fn jail_path(candidate: &Path, jail_root: &Path) -> Result<PathBuf, String> {
    if candidate.to_string_lossy().as_bytes().contains(&0) {
        return Err("path contains null byte".to_string());
    }

    #[cfg(feature = "no-jail")]
    {
        let _ = jail_root;
        return Ok(canonicalize_or_self(candidate));
    }

    #[allow(unreachable_code)]
    {
        let cfg = crate::core::config::Config::load();
        if cfg.path_jail == Some(false) {
            return Ok(canonicalize_or_self(candidate));
        }

        let root = canonicalize_or_self(jail_root);

        // Resolve relative candidates against the (absolute) jail root — never the process
        // CWD. The daemon's CWD is not the project, so CWD-relative resolution made
        // graph-relative paths (e.g. auto-preload candidates like `rust/src/core/foo.rs`)
        // spuriously fail with "no existing ancestor". Absolute candidates are unchanged.
        let resolved: PathBuf;
        let candidate: &Path = if candidate.is_absolute() {
            candidate
        } else {
            resolved = root.join(candidate);
            resolved.as_path()
        };

        let allow = allow_paths_from_env_and_config();

        let (base, remainder) = canonicalize_existing_ancestor(candidate).ok_or_else(|| {
            format!(
                "path does not exist and has no existing ancestor: {}",
                candidate.display()
            )
        })?;

        let allowed =
            is_under_prefix(&base, &root) || allow.iter().any(|p| is_under_prefix(&base, p));

        #[cfg(windows)]
        let allowed = allowed || is_under_prefix_windows(&base, &root);

        if !allowed {
            let base_msg = format!(
                "path escapes project root: {} (root: {})",
                candidate.display(),
                root.display(),
            );
            let hint = if crate::core::protocol::meta_visible() {
                format!(
                ". Hint: set LEAN_CTX_ALLOW_PATH={} or add it to allow_paths in ~/.lean-ctx/config.toml",
                candidate.parent().unwrap_or(candidate).display()
            )
            } else {
                String::new()
            };
            return Err(format!("{base_msg}{hint}"));
        }

        #[cfg(windows)]
        reject_symlink_on_windows(candidate)?;

        let mut out = base;
        for part in remainder.iter().rev() {
            out.push(part);
        }

        // Re-validate after reconstruction: if the final path exists, canonicalize
        // and re-check to close TOCTOU window (symlink created between check and use).
        if out.exists() {
            let final_canon = canonicalize_or_self(&out);
            let final_ok = is_under_prefix(&final_canon, &root)
                || allow.iter().any(|p| is_under_prefix(&final_canon, p));
            #[cfg(windows)]
            let final_ok = final_ok || is_under_prefix_windows(&final_canon, &root);
            if !final_ok {
                return Err(format!(
                    "post-canonicalize jail escape detected: {} resolves to {}",
                    candidate.display(),
                    final_canon.display()
                ));
            }
        }

        Ok(out)
    }
}

#[cfg(windows)]
fn is_under_prefix_windows(path: &Path, prefix: &Path) -> bool {
    let path_str = normalize_windows_path(&path.to_string_lossy());
    let prefix_str = normalize_windows_path(&prefix.to_string_lossy());
    path_str.starts_with(&prefix_str)
}

#[cfg(windows)]
fn normalize_windows_path(s: &str) -> String {
    let stripped = super::pathutil::strip_verbatim_str(s).unwrap_or_else(|| s.to_string());
    stripped.to_lowercase().replace('/', "\\")
}

#[cfg(windows)]
fn reject_symlink_on_windows(path: &Path) -> Result<(), String> {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.is_symlink() {
            return Err(format!(
                "symlink not allowed in jailed path: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "no-jail"))]
    #[test]
    fn rejects_path_outside_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        let other = tmp.path().join("other");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(root.join("a.txt"), "ok").unwrap();
        std::fs::write(other.join("b.txt"), "no").unwrap();

        let ok = jail_path(&root.join("a.txt"), &root);
        assert!(ok.is_ok());

        let bad = jail_path(&other.join("b.txt"), &root);
        assert!(bad.is_err());
    }

    #[test]
    fn allows_nonexistent_child_under_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "ok").unwrap();

        let p = root.join("new").join("file.txt");
        let ok = jail_path(&p, &root).unwrap();
        assert!(ok.to_string_lossy().contains("file.txt"));
    }

    #[cfg(not(feature = "no-jail"))]
    #[test]
    fn relative_candidate_resolves_against_root_not_cwd() {
        // Regression: in the daemon (CWD != project) a relative graph path like
        // `sub/file.rs` must resolve under the jail root, not the process CWD.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("sub").join("file.rs"), "ok").unwrap();

        let jailed = jail_path(Path::new("sub/file.rs"), &root)
            .expect("relative candidate should resolve under the jail root");
        assert!(jailed.ends_with("sub/file.rs"));
        assert!(
            is_under_prefix(&canonicalize_or_self(&jailed), &canonicalize_or_self(&root)),
            "resolved path must live under the jail root: {jailed:?}"
        );
    }

    #[test]
    fn ide_config_dirs_list_is_not_empty() {
        assert!(IDE_CONFIG_DIRS.len() >= 10);
        assert!(IDE_CONFIG_DIRS.contains(&".codex"));
        assert!(IDE_CONFIG_DIRS.contains(&".cursor"));
        assert!(IDE_CONFIG_DIRS.contains(&".claude"));
        assert!(IDE_CONFIG_DIRS.contains(&".gemini"));
    }

    #[test]
    fn canonicalize_or_self_strips_verbatim() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("project");
        std::fs::create_dir_all(&dir).unwrap();

        let result = canonicalize_or_self(&dir);
        let s = result.to_string_lossy();
        assert!(
            !s.starts_with(r"\\?\"),
            "canonicalize_or_self should strip verbatim prefix, got: {s}"
        );
    }

    #[test]
    fn jail_path_accepts_same_dir_different_format() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("file.rs"), "ok").unwrap();

        let result = jail_path(&root.join("file.rs"), &root);
        assert!(result.is_ok(), "same dir should be accepted: {result:?}");
    }

    #[cfg(not(feature = "no-jail"))]
    #[test]
    fn error_message_contains_escape_info() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        let other = tmp.path().join("other");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("b.txt"), "no").unwrap();

        let err = jail_path(&other.join("b.txt"), &root).unwrap_err();
        assert!(
            err.contains("path escapes project root"),
            "error should mention escape: {err}"
        );
    }

    #[test]
    fn allow_path_env_permits_outside_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        let other = tmp.path().join("other");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("b.txt"), "allowed").unwrap();

        let canon = canonicalize_or_self(&other);
        std::env::set_var("LEAN_CTX_ALLOW_PATH", canon.to_string_lossy().as_ref());
        let result = jail_path(&other.join("b.txt"), &root);
        std::env::remove_var("LEAN_CTX_ALLOW_PATH");

        assert!(
            result.is_ok(),
            "LEAN_CTX_ALLOW_PATH should permit access: {result:?}"
        );
    }

    #[cfg(all(unix, not(feature = "no-jail")))]
    #[test]
    fn rejects_symlink_escape_on_unix() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        let other = tmp.path().join("other");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("secret.txt"), "no").unwrap();

        let link = root.join("link.txt");
        symlink(other.join("secret.txt"), &link).unwrap();

        let bad = jail_path(&link, &root);
        assert!(bad.is_err(), "symlink escape must be rejected: {bad:?}");
    }

    #[test]
    fn rejects_null_byte_in_path() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let bad_path = PathBuf::from("file\0.txt");
        let result = jail_path(&bad_path, &root);
        assert!(result.is_err(), "null byte in path must be rejected");
        assert!(
            result.unwrap_err().contains("null byte"),
            "error must mention null byte"
        );
    }
}
