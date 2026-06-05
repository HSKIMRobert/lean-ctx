use std::path::{Path, PathBuf};

/// Canonicalize a path and strip the Windows verbatim/extended-length prefix (`\\?\`)
/// that `std::fs::canonicalize` adds on Windows. This prefix breaks many tools and
/// string-based path comparisons.
///
/// On non-Windows platforms this is equivalent to `std::fs::canonicalize`.
pub fn safe_canonicalize(path: &Path) -> std::io::Result<PathBuf> {
    let canon = std::fs::canonicalize(path)?;
    Ok(strip_verbatim(canon))
}

/// Like `safe_canonicalize` but returns the original path on failure.
pub fn safe_canonicalize_or_self(path: &Path) -> PathBuf {
    safe_canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Canonicalize with a timeout guard. Protects against hangs on WSL2 DrvFS,
/// Windows reparse points, NFS, FUSE, sshfs, and other slow filesystems.
/// Falls back to the original path if canonicalize doesn't complete within the timeout.
/// Self-healing: after a timeout, subsequent calls to slow mounts skip the thread entirely.
pub fn safe_canonicalize_bounded(path: &Path, timeout_ms: u64) -> PathBuf {
    use super::io_health;

    let path_str = path.to_string_lossy();
    if io_health::is_slow_mount(&path_str) && io_health::recent_freeze_count() > 0 {
        return safe_canonicalize_or_self(path);
    }

    let effective_timeout =
        io_health::adaptive_timeout(std::time::Duration::from_millis(timeout_ms));

    let path_owned = path.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("canonicalize-bounded".into())
        .spawn(move || {
            let result = safe_canonicalize(&path_owned).unwrap_or(path_owned);
            let _ = tx.send(result);
        });
    if let Ok(canonical) = rx.recv_timeout(effective_timeout) {
        canonical
    } else {
        io_health::record_freeze();
        tracing::warn!(
            "[SECURITY] canonicalize timed out ({}ms) for {}; PathJail checks on \
             uncanonicalized paths may be less reliable",
            effective_timeout.as_millis(),
            path.display()
        );
        path.to_path_buf()
    }
}

/// Remove the `\\?\` / `//?/` verbatim prefix from a `PathBuf`.
/// Handles both regular verbatim (`\\?\C:\...`) and UNC verbatim (`\\?\UNC\...`).
pub fn strip_verbatim(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = strip_verbatim_str(&s) {
        PathBuf::from(stripped)
    } else {
        path
    }
}

/// Remove the `\\?\` / `//?/` verbatim prefix from a path string.
/// Returns `Some(cleaned)` if a prefix was found, `None` otherwise.
pub fn strip_verbatim_str(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");

    if let Some(rest) = normalized.strip_prefix("//?/UNC/") {
        Some(format!("//{rest}"))
    } else {
        normalized
            .strip_prefix("//?/")
            .map(std::string::ToString::to_string)
    }
}

/// Normalize paths from any client format to a consistent OS-native form.
/// Handles MSYS2/Git Bash (`/c/Users/...` -> `C:/Users/...`), mixed separators,
/// double slashes, and trailing slashes. Uses forward slashes for consistency.
pub fn normalize_tool_path(path: &str) -> String {
    let mut p = match strip_verbatim_str(path) {
        Some(stripped) => stripped,
        None => path.to_string(),
    };

    // MSYS2/Git Bash: /c/Users/... -> C:/Users/...
    if p.len() >= 3
        && p.starts_with('/')
        && p.as_bytes()[1].is_ascii_alphabetic()
        && p.as_bytes()[2] == b'/'
    {
        let drive = p.as_bytes()[1].to_ascii_uppercase() as char;
        p = format!("{drive}:{}", &p[2..]);
    }

    p = p.replace('\\', "/");

    // Collapse double slashes (preserve UNC paths starting with //)
    while p.contains("//") && !p.starts_with("//") {
        p = p.replace("//", "/");
    }

    // Remove trailing slash (unless root like "/" or "C:/")
    if p.len() > 1 && p.ends_with('/') && !p.ends_with(":/") {
        p.pop();
    }

    // Resolve symlinks for absolute paths to ensure cache key consistency.
    // Skip relative paths (preserve "." / "../" as-is), root-only paths (/ or C:/),
    // and slow mounts (WSL DrvFS /mnt/) where canonicalize can hang.
    // Uses safe_canonicalize to strip Windows \\?\ prefix.
    let is_absolute = p.starts_with('/') || (p.len() >= 3 && p.as_bytes()[1] == b':');
    let is_root_only = p == "/" || (p.len() <= 3 && p.ends_with('/') && is_absolute);
    if is_absolute && !is_root_only && !crate::core::io_health::is_slow_mount(&p) {
        if let Ok(canonical) = safe_canonicalize(Path::new(&*p)) {
            let canonical_str = canonical.to_string_lossy().replace('\\', "/");
            if !canonical_str.is_empty() {
                p = canonical_str;
            }
        }
    }

    p
}

/// Returns `true` if the directory is too broad to be a valid project root.
/// Rejects home directory, filesystem root, `.` (bare CWD), and agent sandbox
/// directories (`.claude`, `.codex`). Used to prevent writing project-scoped
/// data (overlays, policies) into the global `~/.lean-ctx/` data directory.
pub fn is_broad_or_unsafe_root(dir: &Path) -> bool {
    if let Some(home) = dirs::home_dir() {
        if dir == home {
            return true;
        }
    }
    let s = dir.to_string_lossy();
    if s == "/" || s == "\\" || s == "." {
        return true;
    }
    s.ends_with("/.claude")
        || s.ends_with("/.codex")
        || s.contains("/.claude/")
        || s.contains("/.codex/")
}

/// Well-known project markers used to identify project roots.
pub const PROJECT_MARKERS: &[&str] = &[
    ".git",
    "Cargo.toml",
    "package.json",
    "go.mod",
    "pyproject.toml",
    "setup.py",
    "pom.xml",
    "build.gradle",
    "Makefile",
    ".lean-ctx.toml",
    ".planning",
];

/// Returns `true` if `dir` contains at least one known project marker.
pub fn has_project_marker(dir: &Path) -> bool {
    PROJECT_MARKERS.iter().any(|m| dir.join(m).exists())
}

/// Returns `true` if `dir` is the home directory or one of the macOS "magic"
/// home subdirectories (`Documents`, `Desktop`, `Downloads`).
///
/// macOS guards these with TCC: the first time a process *enumerates or stats
/// inside* one, the OS pops a privacy prompt ("lean-ctx would like to access
/// files in your Documents folder", #356). They are also never valid project
/// roots or multi-repo workspace parents, so scan heuristics should treat them
/// as off-limits *without* calling `read_dir` (which is what trips the prompt).
pub fn is_tcc_sensitive_home_dir(dir: &Path) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    if dir == home {
        return true;
    }
    if dir.parent() != Some(home.as_path()) {
        return false;
    }
    matches!(
        dir.file_name().and_then(|n| n.to_str()),
        Some("Documents" | "Desktop" | "Downloads")
    )
}

/// Returns `true` if `dir` is a multi-repo workspace parent — i.e. it has at
/// least 2 immediate child directories that each contain a project marker.
pub fn has_multi_repo_children(dir: &Path) -> bool {
    // Never enumerate the home dir or macOS TCC-protected dirs: read_dir there
    // pops a macOS privacy prompt (#356) and they are never workspace parents.
    if is_tcc_sensitive_home_dir(dir) {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let count = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|ft| ft.is_dir()))
        .filter(|e| has_project_marker(&e.path()))
        .take(2)
        .count();
    count >= 2
}

/// Returns `true` if `project_root` collides with the lean-ctx data directory.
/// This prevents project-scoped files (overlays.json, policies.json) from being
/// written into `~/.lean-ctx/` or `~/.config/lean-ctx/`.
pub fn is_data_dir_collision(project_root: &Path) -> bool {
    if is_broad_or_unsafe_root(project_root) {
        return true;
    }
    if let Ok(data_dir) = crate::core::data_dir::lean_ctx_data_dir() {
        let project_lean_ctx = project_root.join(".lean-ctx");
        if project_lean_ctx == data_dir || data_dir.starts_with(&project_lean_ctx) {
            return true;
        }
    }
    false
}

/// Returns the project-scoped `.lean-ctx/` directory if the project root is safe.
/// Returns `Err` if the project root collides with the global data directory.
pub fn safe_project_data_dir(project_root: &Path) -> Result<PathBuf, String> {
    if is_data_dir_collision(project_root) {
        return Err(format!(
            "project root {} collides with global data directory; \
             skipping project-scoped write",
            project_root.display()
        ));
    }
    Ok(project_root.join(".lean-ctx"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_regular_verbatim() {
        let p = PathBuf::from(r"\\?\C:\Users\dev\project");
        let result = strip_verbatim(p);
        assert_eq!(result, PathBuf::from("C:/Users/dev/project"));
    }

    #[test]
    fn tcc_sensitive_home_dir_matches_home_and_magic_dirs() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        // Home itself and the macOS magic dirs are off-limits (#356).
        assert!(is_tcc_sensitive_home_dir(&home));
        assert!(is_tcc_sensitive_home_dir(&home.join("Documents")));
        assert!(is_tcc_sensitive_home_dir(&home.join("Desktop")));
        assert!(is_tcc_sensitive_home_dir(&home.join("Downloads")));
    }

    #[test]
    fn tcc_sensitive_home_dir_allows_real_projects() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        // A real project (even nested under Documents) and non-magic home children
        // are scannable — only the bare magic dirs / home are blocked.
        assert!(!is_tcc_sensitive_home_dir(
            &home.join("Documents").join("my-project")
        ));
        assert!(!is_tcc_sensitive_home_dir(&home.join("code")));
        assert!(!is_tcc_sensitive_home_dir(&home.join("Projects")));
    }

    #[test]
    fn strip_unc_verbatim() {
        let p = PathBuf::from(r"\\?\UNC\server\share\dir");
        let result = strip_verbatim(p);
        assert_eq!(result, PathBuf::from("//server/share/dir"));
    }

    #[test]
    fn no_prefix_unchanged() {
        let p = PathBuf::from("/home/user/project");
        let result = strip_verbatim(p.clone());
        assert_eq!(result, p);
    }

    #[test]
    fn windows_drive_unchanged() {
        let p = PathBuf::from("C:/Users/dev");
        let result = strip_verbatim(p.clone());
        assert_eq!(result, p);
    }

    #[test]
    fn strip_str_regular() {
        assert_eq!(
            strip_verbatim_str(r"\\?\E:\code\lean-ctx"),
            Some("E:/code/lean-ctx".to_string())
        );
    }

    #[test]
    fn strip_str_unc() {
        assert_eq!(
            strip_verbatim_str(r"\\?\UNC\myserver\data"),
            Some("//myserver/data".to_string())
        );
    }

    #[test]
    fn strip_str_forward_slash_variant() {
        assert_eq!(
            strip_verbatim_str("//?/C:/Users/dev"),
            Some("C:/Users/dev".to_string())
        );
    }

    #[test]
    fn strip_str_no_prefix() {
        assert_eq!(strip_verbatim_str("/home/user"), None);
    }

    #[test]
    fn safe_canonicalize_or_self_nonexistent() {
        let p = Path::new("/this/path/should/not/exist/xyzzy");
        let result = safe_canonicalize_or_self(p);
        assert_eq!(result, p.to_path_buf());
    }

    #[test]
    fn normalize_msys_path_to_native() {
        assert_eq!(
            normalize_tool_path("/c/Users/ABC/AppData/lean-ctx"),
            "C:/Users/ABC/AppData/lean-ctx"
        );
    }

    #[test]
    fn normalize_msys_uppercase_drive() {
        assert_eq!(
            normalize_tool_path("/D/Program Files/lean-ctx.exe"),
            "D:/Program Files/lean-ctx.exe"
        );
    }

    #[test]
    fn normalize_native_windows_path_unchanged() {
        assert_eq!(
            normalize_tool_path("C:/Users/ABC/lean-ctx.exe"),
            "C:/Users/ABC/lean-ctx.exe"
        );
    }

    #[test]
    fn normalize_backslash_windows_path() {
        assert_eq!(
            normalize_tool_path(r"C:\Users\ABC\lean-ctx.exe"),
            "C:/Users/ABC/lean-ctx.exe"
        );
    }

    #[test]
    fn normalize_unix_path_unchanged() {
        assert_eq!(
            normalize_tool_path("/usr/local/bin/lean-ctx"),
            "/usr/local/bin/lean-ctx"
        );
    }

    #[test]
    fn normalize_windows_path_with_spaces_and_backslashes() {
        // The exact "paths with spaces" scenario reported on Windows (#324):
        // backslashes are converted to forward slashes (so client render layers
        // never escape-mangle them) while spaces in directory names survive.
        assert_eq!(
            normalize_tool_path(r"C:\Users\My Name\My Project\src\main.rs"),
            "C:/Users/My Name/My Project/src/main.rs"
        );
        assert_eq!(
            normalize_tool_path(r"C:\Program Files\app\config.toml"),
            "C:/Program Files/app/config.toml"
        );
    }

    #[test]
    fn normalize_double_slashes() {
        assert_eq!(
            normalize_tool_path("C:/Users//ABC//lean-ctx"),
            "C:/Users/ABC/lean-ctx"
        );
    }

    #[test]
    fn normalize_trailing_slash_removed() {
        assert_eq!(normalize_tool_path("/c/Users/ABC/"), "C:/Users/ABC");
    }

    #[test]
    fn normalize_root_slash_preserved() {
        assert_eq!(normalize_tool_path("/"), "/");
    }

    #[test]
    fn normalize_drive_root_preserved() {
        assert_eq!(normalize_tool_path("C:/"), "C:/");
    }

    #[test]
    fn normalize_verbatim_with_msys() {
        assert_eq!(normalize_tool_path(r"\\?\C:\Users\dev"), "C:/Users/dev");
    }

    #[test]
    fn broad_root_rejects_home() {
        if let Some(home) = dirs::home_dir() {
            assert!(is_broad_or_unsafe_root(&home));
        }
    }

    #[test]
    fn broad_root_rejects_filesystem_root() {
        assert!(is_broad_or_unsafe_root(Path::new("/")));
    }

    #[test]
    fn broad_root_rejects_dot() {
        assert!(is_broad_or_unsafe_root(Path::new(".")));
    }

    #[test]
    fn broad_root_rejects_agent_dirs() {
        assert!(is_broad_or_unsafe_root(Path::new("/home/user/.claude")));
        assert!(is_broad_or_unsafe_root(Path::new("/home/user/.codex")));
    }

    #[test]
    fn broad_root_allows_project_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("my-project");
        std::fs::create_dir_all(&subdir).unwrap();
        assert!(!is_broad_or_unsafe_root(&subdir));
    }

    #[test]
    fn broad_root_allows_home_subdirs() {
        if let Some(home) = dirs::home_dir() {
            let subdir = home.join("projects").join("my-app");
            assert!(!is_broad_or_unsafe_root(&subdir));
        }
    }

    #[test]
    fn data_dir_collision_rejects_home() {
        if let Some(home) = dirs::home_dir() {
            assert!(is_data_dir_collision(&home));
        }
    }

    #[test]
    fn data_dir_collision_allows_normal_project() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("my-project");
        std::fs::create_dir_all(&project).unwrap();
        assert!(!is_data_dir_collision(&project));
    }

    #[test]
    fn has_project_marker_detects_git() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("repo");
        std::fs::create_dir_all(&root).unwrap();
        assert!(!has_project_marker(&root));
        std::fs::create_dir(root.join(".git")).unwrap();
        assert!(has_project_marker(&root));
    }

    #[test]
    fn has_project_marker_detects_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("rust-project");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]").unwrap();
        assert!(has_project_marker(&root));
    }

    #[test]
    fn multi_repo_children_needs_two() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path().join("code");
        std::fs::create_dir_all(&parent).unwrap();

        // 0 repos → false
        assert!(!has_multi_repo_children(&parent));

        // 1 repo → false
        let repo1 = parent.join("repo1");
        std::fs::create_dir_all(repo1.join(".git")).unwrap();
        assert!(!has_multi_repo_children(&parent));

        // 2 repos → true
        let repo2 = parent.join("repo2");
        std::fs::create_dir_all(repo2.join(".git")).unwrap();
        assert!(has_multi_repo_children(&parent));
    }

    #[test]
    fn multi_repo_children_ignores_files() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path().join("mixed");
        std::fs::create_dir_all(&parent).unwrap();

        // One repo dir + one plain file with .git name (not a dir)
        let repo1 = parent.join("repo1");
        std::fs::create_dir_all(repo1.join(".git")).unwrap();
        std::fs::write(parent.join("not-a-repo"), "file").unwrap();
        assert!(!has_multi_repo_children(&parent));

        // Add second actual repo
        let repo2 = parent.join("repo2");
        std::fs::create_dir_all(&repo2).unwrap();
        std::fs::write(repo2.join("package.json"), "{}").unwrap();
        assert!(has_multi_repo_children(&parent));
    }

    #[test]
    fn multi_repo_children_nonexistent_dir() {
        assert!(!has_multi_repo_children(Path::new("/nonexistent/path/xyz")));
    }
}
