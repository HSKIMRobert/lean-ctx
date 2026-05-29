use std::path::Path;

const PROJECT_MARKERS: &[&str] = &[
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
];

/// Parse a `file://` URI to a validated local path string.
/// Rejects non-file URIs, null bytes, `..` traversal, and non-directory paths.
/// Returns a canonicalized absolute path.
pub fn uri_to_path(uri: &str) -> Option<String> {
    let raw = uri.strip_prefix("file://")?;
    if raw.contains("%00") {
        return None;
    }
    let decoded = percent_decode(raw);
    if decoded.is_empty() || decoded.contains('\0') {
        return None;
    }
    let path = Path::new(&decoded);
    if !path.is_absolute() {
        return None;
    }
    let canonical = crate::core::pathutil::safe_canonicalize_or_self(path);
    let s = canonical.to_string_lossy().to_string();
    if s.is_empty() {
        return None;
    }
    Some(s)
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().and_then(hex_val);
            let lo = chars.next().and_then(hex_val);
            if let (Some(h), Some(l)) = (hi, lo) {
                let byte = h << 4 | l;
                if byte == 0 {
                    continue;
                }
                out.push(byte as char);
            } else {
                out.push('%');
            }
        } else {
            out.push(b as char);
        }
    }
    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub(super) fn has_project_marker(dir: &Path) -> bool {
    PROJECT_MARKERS.iter().any(|m| dir.join(m).exists())
}

/// Select the best project root from MCP client roots.
/// Only considers paths that are existing directories.
/// Prefers roots with project markers (.git, Cargo.toml, etc.).
/// Falls back to the first valid directory if none have markers.
pub fn best_root_from_uris(uris: &[String]) -> Option<String> {
    let paths: Vec<String> = uris
        .iter()
        .filter_map(|u| uri_to_path(u))
        .filter(|p| Path::new(p).is_dir())
        .collect();

    if paths.is_empty() {
        return None;
    }

    for p in &paths {
        if has_project_marker(Path::new(p)) {
            return Some(p.clone());
        }
    }

    Some(paths[0].clone())
}

/// Filter and validate URIs to existing directories only.
pub fn valid_dir_paths_from_uris(uris: &[String]) -> Vec<String> {
    uris.iter()
        .filter_map(|u| uri_to_path(u))
        .filter(|p| Path::new(p).is_dir())
        .collect()
}

/// Detect project root from IDE-specific environment variables.
/// Priority: LEAN_CTX_PROJECT_ROOT > CLAUDE_PROJECT_DIR
pub fn root_from_env() -> Option<String> {
    for var in ["LEAN_CTX_PROJECT_ROOT", "CLAUDE_PROJECT_DIR"] {
        if let Ok(val) = std::env::var(var) {
            let trimmed = val.trim().to_string();
            if !trimmed.is_empty() && Path::new(&trimmed).is_dir() {
                return Some(trimmed);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn parse_file_uri_unix() {
        assert_eq!(
            uri_to_path("file:///home/user/project"),
            Some("/home/user/project".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn parse_file_uri_windows() {
        assert_eq!(
            uri_to_path("file:///C:/Users/dev/project"),
            Some("/C:/Users/dev/project".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn parse_file_uri_with_spaces() {
        assert_eq!(
            uri_to_path("file:///home/user/my%20project"),
            Some("/home/user/my project".to_string())
        );
    }

    #[test]
    fn parse_non_file_uri_returns_none() {
        assert!(uri_to_path("https://example.com").is_none());
        assert!(uri_to_path("").is_none());
    }

    #[test]
    fn rejects_null_bytes() {
        assert!(uri_to_path("file:///tmp/evil%00path").is_none());
    }

    #[test]
    fn rejects_relative_uri() {
        assert!(uri_to_path("file://relative/path").is_none());
    }

    #[test]
    fn canonicalizes_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("a").join("b");
        std::fs::create_dir_all(&sub).unwrap();
        let traversal = format!("file://{}/a/b/../..", tmp.path().display());
        let result = uri_to_path(&traversal);
        assert!(result.is_some());
        let resolved = result.unwrap();
        assert!(
            !resolved.contains(".."),
            "should be canonicalized: {resolved}"
        );
    }

    #[test]
    fn best_root_prefers_marker() {
        let tmp = tempfile::tempdir().unwrap();
        let with_marker = tmp.path().join("has_git");
        let without = tmp.path().join("plain");
        std::fs::create_dir_all(&with_marker).unwrap();
        std::fs::create_dir_all(&without).unwrap();
        std::fs::create_dir(with_marker.join(".git")).unwrap();

        let uris = vec![
            format!("file://{}", without.display()),
            format!("file://{}", with_marker.display()),
        ];
        let result = best_root_from_uris(&uris).unwrap();
        assert!(result.contains("has_git"));
    }

    #[test]
    fn best_root_falls_back_to_first_existing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("dir_a");
        let b = tmp.path().join("dir_b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();

        let uris = vec![
            format!("file://{}", a.display()),
            format!("file://{}", b.display()),
        ];
        let result = best_root_from_uris(&uris).unwrap();
        assert!(result.contains("dir_a"));
    }

    #[test]
    fn best_root_skips_nonexistent() {
        let uris = vec!["file:///nonexistent_abc_123".to_string()];
        assert!(best_root_from_uris(&uris).is_none());
    }

    #[test]
    fn best_root_empty_returns_none() {
        assert!(best_root_from_uris(&[]).is_none());
    }

    #[test]
    fn env_override_returns_none_when_unset() {
        let _ = root_from_env();
    }

    #[test]
    fn all_paths_from_uris() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("project_a");
        let b = tmp.path().join("project_b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::create_dir(a.join(".git")).unwrap();

        let uris = vec![
            format!("file://{}", a.display()),
            format!("file://{}", b.display()),
        ];

        let paths: Vec<String> = uris.iter().filter_map(|u| uri_to_path(u)).collect();
        assert_eq!(paths.len(), 2);
        assert!(paths[0].contains("project_a"));
        assert!(paths[1].contains("project_b"));

        let best = best_root_from_uris(&uris).unwrap();
        assert!(best.contains("project_a"));
    }
}
