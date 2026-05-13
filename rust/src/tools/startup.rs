use super::server::LeanCtxServer;

#[derive(Clone, Debug, Default)]
pub(super) struct StartupContext {
    pub(super) project_root: Option<String>,
    pub(super) shell_cwd: Option<String>,
}

/// Creates a new `LeanCtxServer` with default configuration.
pub fn create_server() -> LeanCtxServer {
    LeanCtxServer::new()
}

pub(super) const PROJECT_ROOT_MARKERS: &[&str] = &[
    ".git",
    ".lean-ctx.toml",
    "Cargo.toml",
    "package.json",
    "go.mod",
    "pyproject.toml",
    "pom.xml",
    "build.gradle",
    "Makefile",
    ".planning",
];

pub(super) fn has_project_marker(dir: &std::path::Path) -> bool {
    PROJECT_ROOT_MARKERS.iter().any(|m| dir.join(m).exists())
}

pub(super) fn is_suspicious_root(dir: &std::path::Path) -> bool {
    let s = dir.to_string_lossy();
    s.contains("/.claude")
        || s.contains("/.codex")
        || s.contains("\\.claude")
        || s.contains("\\.codex")
}

pub(super) fn canonicalize_path(path: &std::path::Path) -> String {
    crate::core::pathutil::safe_canonicalize_or_self(path)
        .to_string_lossy()
        .to_string()
}

pub(super) fn detect_startup_context(
    explicit_project_root: Option<&str>,
    startup_cwd: Option<&std::path::Path>,
) -> StartupContext {
    let shell_cwd = startup_cwd.map(canonicalize_path);
    let project_root = explicit_project_root
        .map(|root| canonicalize_path(std::path::Path::new(root)))
        .or_else(|| {
            startup_cwd
                .and_then(maybe_derive_project_root_from_absolute)
                .map(|p| canonicalize_path(&p))
        });

    let shell_cwd = match (shell_cwd, project_root.as_ref()) {
        (Some(cwd), Some(root))
            if std::path::Path::new(&cwd).starts_with(std::path::Path::new(root)) =>
        {
            Some(cwd)
        }
        (_, Some(root)) => Some(root.clone()),
        (cwd, None) => cwd,
    };

    StartupContext {
        project_root,
        shell_cwd,
    }
}

pub(super) fn maybe_derive_project_root_from_absolute(
    abs: &std::path::Path,
) -> Option<std::path::PathBuf> {
    let mut cur = if abs.is_dir() {
        abs.to_path_buf()
    } else {
        abs.parent()?.to_path_buf()
    };
    loop {
        if has_project_marker(&cur) {
            return Some(crate::core::pathutil::safe_canonicalize_or_self(&cur));
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

pub(super) fn auto_consolidate_knowledge(project_root: &str) {
    use crate::core::knowledge::ProjectKnowledge;
    use crate::core::session::SessionState;

    let Some(session) = SessionState::load_latest() else {
        return;
    };

    if session.findings.is_empty() && session.decisions.is_empty() {
        return;
    }

    let Ok(policy) = crate::core::config::Config::load().memory_policy_effective() else {
        return;
    };
    let mut knowledge = ProjectKnowledge::load_or_create(project_root);

    for finding in &session.findings {
        let key = if let Some(ref file) = finding.file {
            if let Some(line) = finding.line {
                format!("{file}:{line}")
            } else {
                file.clone()
            }
        } else {
            "finding-auto".to_string()
        };
        knowledge.remember("finding", &key, &finding.summary, &session.id, 0.7, &policy);
    }

    for decision in &session.decisions {
        let key = decision
            .summary
            .chars()
            .take(50)
            .collect::<String>()
            .replace(' ', "-")
            .to_lowercase();
        knowledge.remember(
            "decision",
            &key,
            &decision.summary,
            &session.id,
            0.85,
            &policy,
        );
    }

    let task_desc = session
        .task
        .as_ref()
        .map(|t| t.description.clone())
        .unwrap_or_default();

    let summary = format!(
        "Auto-consolidate session {}: {} — {} findings, {} decisions",
        session.id,
        task_desc,
        session.findings.len(),
        session.decisions.len()
    );
    knowledge.consolidate(&summary, vec![session.id.clone()], &policy);
    let _ = knowledge.save();
}
