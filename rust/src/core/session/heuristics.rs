use super::types::SessionState;

pub(crate) fn normalize_loaded_session(mut session: SessionState) -> SessionState {
    if matches!(session.project_root.as_deref(), Some(r) if r.trim().is_empty()) {
        session.project_root = None;
    }
    if matches!(session.shell_cwd.as_deref(), Some(c) if c.trim().is_empty()) {
        session.shell_cwd = None;
    }

    if let (Some(ref root), Some(ref cwd)) = (&session.project_root, &session.shell_cwd) {
        let root_p = std::path::Path::new(root);
        let cwd_p = std::path::Path::new(cwd);
        let root_looks_real = has_project_marker(root_p);
        let cwd_looks_real = has_project_marker(cwd_p);

        if !root_looks_real && cwd_looks_real && is_agent_or_temp_dir(root_p) {
            session.project_root = Some(cwd.clone());
        }
    }

    if session.compression_level.is_empty() {
        if session.terse_mode {
            session.compression_level = "lite".to_string();
        } else if let Some(env_level) = crate::core::config::CompressionLevel::from_env() {
            session.compression_level = env_level.label().to_string();
            session.terse_mode = env_level.is_active();
        } else {
            let profile = crate::core::profiles::active_profile();
            if profile.compression.terse_mode_effective() {
                session.compression_level = "lite".to_string();
                session.terse_mode = true;
            } else {
                session.compression_level = "off".to_string();
            }
        }
    } else if !session.terse_mode {
        let level =
            crate::core::config::CompressionLevel::from_str_label(&session.compression_level)
                .unwrap_or_default();
        session.terse_mode = level.is_active();
    }

    session
}

pub(crate) fn session_matches_project_root(
    session: &SessionState,
    target_root: &std::path::Path,
) -> bool {
    if let Some(root) = session.project_root.as_deref() {
        let root_path =
            crate::core::pathutil::safe_canonicalize_or_self(std::path::Path::new(root));
        if root_path == target_root {
            return true;
        }
        if has_project_marker(&root_path) {
            return false;
        }
    }

    if let Some(cwd) = session.shell_cwd.as_deref() {
        let cwd_path = crate::core::pathutil::safe_canonicalize_or_self(std::path::Path::new(cwd));
        return cwd_path == target_root || cwd_path.starts_with(target_root);
    }

    false
}

fn has_project_marker(dir: &std::path::Path) -> bool {
    const MARKERS: &[&str] = &[
        ".git",
        ".lean-ctx.toml",
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pyproject.toml",
        ".planning",
    ];
    MARKERS.iter().any(|m| dir.join(m).exists())
}

fn is_agent_or_temp_dir(dir: &std::path::Path) -> bool {
    let s = dir.to_string_lossy();
    s.contains("/.claude")
        || s.contains("/.codex")
        || s.contains("/var/folders/")
        || s.contains("/tmp/")
        || s.contains("\\.claude")
        || s.contains("\\.codex")
        || s.contains("\\AppData\\Local\\Temp")
        || s.contains("\\Temp\\")
}
