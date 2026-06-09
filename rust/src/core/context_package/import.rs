//! Import (resume) a context package into the current session (#293).

use std::path::Path;

use super::bundle::ContextPackage;
use crate::core::session::SessionState;

/// Resume a session from a previously saved context package.
///
/// Merges the package's session slice, summaries, and knowledge facts into
/// the provided live session. Does **not** overwrite; it augments.
pub fn resume_package(session: &mut SessionState, path: &Path) -> Result<ResumeReport, String> {
    let json = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
    let pkg: ContextPackage = serde_json::from_str(&json).map_err(|e| format!("parse: {e}"))?;

    if !pkg.is_compatible() {
        return Err(format!(
            "package format_version {} is newer than supported ({})",
            pkg.format_version,
            super::bundle::FORMAT_VERSION
        ));
    }

    let mut report = ResumeReport::default();

    // Restore task if not already set.
    if session.task.is_none() {
        session.task.clone_from(&pkg.session.task);
        report.task_restored = session.task.is_some();
    }

    // Merge decisions (deduplicate by summary text).
    let existing_decisions: std::collections::HashSet<String> = session
        .decisions
        .iter()
        .map(|d| d.summary.clone())
        .collect();
    for d in &pkg.session.decisions {
        if !existing_decisions.contains(&d.summary) {
            session.decisions.push(d.clone());
            report.decisions_merged += 1;
        }
    }

    // Merge findings.
    let existing_findings: std::collections::HashSet<String> =
        session.findings.iter().map(|f| f.summary.clone()).collect();
    for f in &pkg.session.findings {
        if !existing_findings.contains(&f.summary) {
            session.findings.push(f.clone());
            report.findings_merged += 1;
        }
    }

    // Merge files (update or insert).
    for pf in &pkg.session.files {
        if !session.files_touched.iter().any(|f| f.path == pf.path) {
            session.files_touched.push(pf.clone());
            report.files_merged += 1;
        }
    }

    // Merge next_steps.
    let existing_next: std::collections::HashSet<String> =
        session.next_steps.iter().cloned().collect();
    for ns in &pkg.session.next_steps {
        if !existing_next.contains(ns) {
            session.next_steps.push(ns.clone());
            report.next_steps_merged += 1;
        }
    }

    // Restore test snapshot if none active.
    if session.test_results.is_none() {
        session.test_results.clone_from(&pkg.session.test_results);
    }

    // Replay knowledge facts into the project knowledge store.
    if !pkg.knowledge.is_empty() {
        report.knowledge_merged = replay_knowledge(&pkg)?;
    }

    // Replay summaries into the summary store.
    if !pkg.summaries.is_empty() {
        report.summaries_merged = replay_summaries(&pkg)?;
    }

    report.source_session_id = pkg.session_id;
    Ok(report)
}

/// Report of what was merged.
#[derive(Debug, Clone, Default)]
pub struct ResumeReport {
    pub source_session_id: String,
    pub task_restored: bool,
    pub decisions_merged: usize,
    pub findings_merged: usize,
    pub files_merged: usize,
    pub next_steps_merged: usize,
    pub knowledge_merged: usize,
    pub summaries_merged: usize,
}

impl ResumeReport {
    pub fn format(&self) -> String {
        let short = self
            .source_session_id
            .split('-')
            .next()
            .unwrap_or(&self.source_session_id);
        let mut lines = vec![format!("resumed from package [{}]", short)];
        if self.task_restored {
            lines.push("  + task restored".to_string());
        }
        if self.decisions_merged > 0 {
            lines.push(format!("  + {} decisions", self.decisions_merged));
        }
        if self.findings_merged > 0 {
            lines.push(format!("  + {} findings", self.findings_merged));
        }
        if self.files_merged > 0 {
            lines.push(format!("  + {} files", self.files_merged));
        }
        if self.next_steps_merged > 0 {
            lines.push(format!("  + {} next steps", self.next_steps_merged));
        }
        if self.knowledge_merged > 0 {
            lines.push(format!("  + {} knowledge facts", self.knowledge_merged));
        }
        if self.summaries_merged > 0 {
            lines.push(format!("  + {} summaries", self.summaries_merged));
        }
        lines.join("\n")
    }
}

fn replay_knowledge(pkg: &ContextPackage) -> Result<usize, String> {
    let mut pk = crate::core::knowledge::ProjectKnowledge::load(&pkg.project_root)
        .unwrap_or_else(|| crate::core::knowledge::ProjectKnowledge::new(&pkg.project_root));
    let policy = crate::core::memory_policy::MemoryPolicy::default();
    let before = pk.facts.len();
    for fact in &pkg.knowledge {
        pk.remember(
            &fact.category,
            &fact.key,
            &fact.value,
            &pkg.session_id,
            fact.confidence,
            &policy,
        );
    }
    pk.save().map_err(|e| format!("save knowledge: {e}"))?;
    Ok(pk.facts.len().saturating_sub(before))
}

fn replay_summaries(pkg: &ContextPackage) -> Result<usize, String> {
    let mut store =
        crate::core::session_summary::store::SummaryStore::load_or_create(&pkg.project_root);
    let before = store.summaries.len();
    let existing_ids: std::collections::HashSet<String> =
        store.summaries.iter().map(|s| s.id.clone()).collect();
    let cfg = crate::core::config::Config::load();
    for s in &pkg.summaries {
        if !existing_ids.contains(&s.id) {
            store.push(s.clone(), cfg.summaries.max_kept as usize);
        }
    }
    store.save().map_err(|e| format!("save summaries: {e}"))?;
    Ok(store.summaries.len().saturating_sub(before))
}
