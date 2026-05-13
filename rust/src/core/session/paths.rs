use chrono::Utc;
use std::path::{Path, PathBuf};

pub(crate) fn escape_xml_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn file_stem_search_pattern(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.chars().any(char::is_alphanumeric))
        .unwrap_or("")
        .to_string()
}

pub(crate) fn parent_dir_slash(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .map_or_else(
            || "./".to_string(),
            |p| {
                let norm = p.replace('\\', "/");
                let trimmed = norm.trim_end_matches('/');
                if trimmed.is_empty() {
                    "./".to_string()
                } else {
                    format!("{trimmed}/")
                }
            },
        )
}

pub(crate) fn sessions_dir() -> Option<PathBuf> {
    crate::core::data_dir::lean_ctx_data_dir()
        .ok()
        .map(|d| d.join("sessions"))
}

pub(crate) fn generate_session_id() -> String {
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let now = Utc::now();
    let ts = now.format("%Y%m%d-%H%M%S").to_string();
    let nanos = now.timestamp_subsec_micros();
    let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{ts}-{nanos:06}s{seq}")
}

/// Extracts the `cd` target from a command string.
/// Handles patterns like `cd /foo`, `cd foo && bar`, `cd ../dir; cmd`, etc.
pub(crate) fn extract_cd_target(command: &str, base_cwd: &str) -> Option<String> {
    let first_cmd = command
        .split("&&")
        .next()
        .unwrap_or(command)
        .split(';')
        .next()
        .unwrap_or(command)
        .trim();

    if !first_cmd.starts_with("cd ") && first_cmd != "cd" {
        return None;
    }

    let target = first_cmd.strip_prefix("cd")?.trim();
    if target.is_empty() || target == "~" {
        return dirs::home_dir().map(|h| h.to_string_lossy().to_string());
    }

    let target = target.trim_matches('"').trim_matches('\'');
    let path = std::path::Path::new(target);

    if path.is_absolute() {
        Some(target.to_string())
    } else {
        let base = std::path::Path::new(base_cwd);
        let joined = base.join(target).to_string_lossy().to_string();
        Some(joined.replace('\\', "/"))
    }
}

pub(crate) fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 2 {
        return path.to_string();
    }
    let last_two: Vec<&str> = parts.iter().rev().take(2).copied().collect();
    format!("…/{}/{}", last_two[1], last_two[0])
}
