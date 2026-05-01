use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Default)]
pub struct LinkedProjects {
    pub roots: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub source: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct WorkspaceConfigFile {
    #[serde(default, rename = "linkedProjects", alias = "linked_projects")]
    linked_projects: Vec<String>,
}

pub fn load_linked_projects(project_root: &Path) -> LinkedProjects {
    let mut out = LinkedProjects::default();

    let Some((source, content)) = read_config_file(project_root) else {
        return out;
    };
    out.source = Some(source.clone());

    let cfg: WorkspaceConfigFile = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            out.warnings.push(format!(
                "workspace config parse failed ({}): {e}",
                source.display()
            ));
            return out;
        }
    };

    let root_canon = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());

    for raw in cfg.linked_projects {
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }

        let candidate = if Path::new(s).is_absolute() {
            PathBuf::from(s)
        } else {
            project_root.join(s)
        };

        let Ok(abs) = candidate.canonicalize() else {
            out.warnings.push(format!(
                "linked project missing/unreadable: {}",
                candidate.to_string_lossy()
            ));
            continue;
        };
        if abs == root_canon {
            continue;
        }
        if !abs.is_dir() {
            out.warnings.push(format!(
                "linked project is not a directory: {}",
                abs.display()
            ));
            continue;
        }
        out.roots.push(abs);
    }

    out.roots.sort();
    out.roots.dedup();
    out
}

fn read_config_file(project_root: &Path) -> Option<(PathBuf, String)> {
    let lean = project_root.join(".leanctx.json");
    if let Ok(s) = std::fs::read_to_string(&lean) {
        return Some((lean, s));
    }
    let socrati = project_root.join(".socraticode.json");
    if let Ok(s) = std::fs::read_to_string(&socrati) {
        return Some((socrati, s));
    }
    None
}
