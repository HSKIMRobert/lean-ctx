use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PropertyGraphMetaV1 {
    pub schema_version: u32,
    /// RFC3339 timestamp (UTC) of the last successful build.
    pub built_at: String,
    /// Git HEAD (short) at build time, if available.
    pub git_head: Option<String>,
    /// Git dirty flag at build time, if available.
    pub git_dirty: Option<bool>,
    /// Node count after build.
    pub nodes: Option<usize>,
    /// Edge count after build.
    pub edges: Option<usize>,
    /// Number of source files processed during build (before filtering).
    pub files_indexed: Option<usize>,
    /// Build duration in milliseconds (best-effort).
    pub build_time_ms: Option<u64>,
}

impl Default for PropertyGraphMetaV1 {
    fn default() -> Self {
        Self {
            schema_version: 1,
            built_at: String::new(),
            git_head: None,
            git_dirty: None,
            nodes: None,
            edges: None,
            files_indexed: None,
            build_time_ms: None,
        }
    }
}

pub fn meta_path(project_root: &Path) -> PathBuf {
    project_root.join(".lean-ctx").join("graph.meta.json")
}

pub fn load_meta(project_root: &Path) -> Option<PropertyGraphMetaV1> {
    let path = meta_path(project_root);
    let s = std::fs::read_to_string(path).ok()?;
    let meta: PropertyGraphMetaV1 = serde_json::from_str(&s).ok()?;
    if meta.schema_version != 1 || meta.built_at.trim().is_empty() {
        return None;
    }
    Some(meta)
}

pub fn write_meta(project_root: &Path, meta: &PropertyGraphMetaV1) -> Result<PathBuf, String> {
    let path = meta_path(project_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    crate::config_io::write_atomic(&path, &json)?;
    Ok(path)
}
