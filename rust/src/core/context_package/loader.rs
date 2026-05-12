use crate::core::knowledge::ProjectKnowledge;
use crate::core::memory_policy::MemoryPolicy;
use crate::core::property_graph::{CodeGraph, Edge, EdgeKind, Node, NodeKind};

use super::content::{GraphLayer, KnowledgeLayer, PackageContent};
use super::manifest::PackageManifest;

#[derive(Debug, Clone, Default)]
pub struct LoadReport {
    pub package_name: String,
    pub package_version: String,
    pub knowledge_facts_merged: u32,
    pub knowledge_facts_skipped: u32,
    pub knowledge_patterns_merged: u32,
    pub graph_nodes_imported: u32,
    pub graph_edges_imported: u32,
    pub gotchas_imported: u32,
    pub warnings: Vec<String>,
}

impl std::fmt::Display for LoadReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Package: {} v{}",
            self.package_name, self.package_version
        )?;
        if self.knowledge_facts_merged > 0 || self.knowledge_facts_skipped > 0 {
            writeln!(
                f,
                "  Knowledge: {} facts merged, {} skipped (duplicates)",
                self.knowledge_facts_merged, self.knowledge_facts_skipped
            )?;
        }
        if self.knowledge_patterns_merged > 0 {
            writeln!(
                f,
                "  Patterns:  {} imported",
                self.knowledge_patterns_merged
            )?;
        }
        if self.graph_nodes_imported > 0 || self.graph_edges_imported > 0 {
            writeln!(
                f,
                "  Graph:     {} nodes, {} edges imported",
                self.graph_nodes_imported, self.graph_edges_imported
            )?;
        }
        if self.gotchas_imported > 0 {
            writeln!(f, "  Gotchas:   {} imported", self.gotchas_imported)?;
        }
        for w in &self.warnings {
            writeln!(f, "  WARNING: {w}")?;
        }
        Ok(())
    }
}

pub fn load_package(
    manifest: &PackageManifest,
    content: &PackageContent,
    project_root: &str,
) -> Result<LoadReport, String> {
    let mut report = LoadReport {
        package_name: manifest.name.clone(),
        package_version: manifest.version.clone(),
        ..Default::default()
    };

    if let Some(ref kl) = content.knowledge {
        merge_knowledge(kl, project_root, manifest, &mut report)?;
    }

    if let Some(ref gl) = content.graph {
        import_graph(gl, project_root, &mut report)?;
    }

    if let Some(ref gotchas) = content.gotchas {
        import_gotchas(gotchas, project_root, &mut report);
    }

    Ok(report)
}

fn merge_knowledge(
    layer: &KnowledgeLayer,
    project_root: &str,
    manifest: &PackageManifest,
    report: &mut LoadReport,
) -> Result<(), String> {
    let mut knowledge = ProjectKnowledge::load_or_create(project_root);
    let policy = MemoryPolicy::default();
    let source_tag = format!("{}@{}", manifest.name, manifest.version);

    for fact in &layer.facts {
        let exists = knowledge
            .facts
            .iter()
            .any(|f| f.category == fact.category && f.key == fact.key && f.value == fact.value);

        if exists {
            report.knowledge_facts_skipped += 1;
            continue;
        }

        knowledge.remember(
            &fact.category,
            &fact.key,
            &fact.value,
            &fact.source_session,
            fact.confidence.min(0.8),
            &policy,
        );
        if let Some(last) = knowledge.facts.last_mut() {
            last.imported_from = Some(source_tag.clone());
        }
        report.knowledge_facts_merged += 1;
    }

    for pattern in &layer.patterns {
        let exists = knowledge.patterns.iter().any(|p| {
            p.pattern_type == pattern.pattern_type && p.description == pattern.description
        });

        if !exists {
            knowledge.patterns.push(pattern.clone());
            report.knowledge_patterns_merged += 1;
        }
    }

    knowledge.save()?;
    Ok(())
}

fn import_graph(
    layer: &GraphLayer,
    project_root: &str,
    report: &mut LoadReport,
) -> Result<(), String> {
    let graph = CodeGraph::open(project_root).map_err(|e| format!("graph open: {e}"))?;

    for node_export in &layer.nodes {
        let node = Node {
            id: None,
            kind: NodeKind::parse(&node_export.kind),
            name: node_export.name.clone(),
            file_path: node_export.file_path.clone(),
            line_start: node_export.line_start,
            line_end: node_export.line_end,
            metadata: node_export.metadata.clone(),
        };

        match graph.upsert_node(&node) {
            Ok(_) => report.graph_nodes_imported += 1,
            Err(e) => {
                report
                    .warnings
                    .push(format!("node import failed ({}): {e}", node_export.name));
            }
        }
    }

    for edge_export in &layer.edges {
        let source = graph
            .get_node_by_path(&edge_export.source_path)
            .map_err(|e| e.to_string())?;
        let target = graph
            .get_node_by_path(&edge_export.target_path)
            .map_err(|e| e.to_string())?;

        if let (Some(src), Some(tgt)) = (source, target) {
            let edge = Edge {
                id: None,
                source_id: src.id.unwrap_or(0),
                target_id: tgt.id.unwrap_or(0),
                kind: EdgeKind::parse(&edge_export.kind),
                metadata: edge_export.metadata.clone(),
            };

            match graph.upsert_edge(&edge) {
                Ok(()) => report.graph_edges_imported += 1,
                Err(e) => {
                    report.warnings.push(format!(
                        "edge import failed ({} -> {}): {e}",
                        edge_export.source_name, edge_export.target_name
                    ));
                }
            }
        }
    }

    Ok(())
}

fn import_gotchas(
    layer: &super::content::GotchasLayer,
    project_root: &str,
    report: &mut LoadReport,
) {
    use crate::core::gotcha_tracker::{
        Gotcha, GotchaCategory, GotchaSeverity, GotchaSource, GotchaStore,
    };

    let mut store = GotchaStore::load(project_root);
    let before = store.gotchas.len();

    for g in &layer.gotchas {
        let dup = store.gotchas.iter().any(|e| e.id == g.id);
        if dup {
            continue;
        }

        let category = GotchaCategory::from_str_loose(&g.category);
        let severity = match g.severity.as_str() {
            "critical" => GotchaSeverity::Critical,
            "warning" => GotchaSeverity::Warning,
            _ => GotchaSeverity::Info,
        };

        let mut gotcha = Gotcha::new(
            category,
            severity,
            &g.trigger,
            &g.resolution,
            GotchaSource::AgentReported {
                session_id: "package-import".into(),
            },
            "package-import",
        );
        g.id.clone_into(&mut gotcha.id);
        g.file_patterns.clone_into(&mut gotcha.file_patterns);
        gotcha.confidence = g.confidence.min(0.8);

        store.gotchas.push(gotcha);
    }

    report.gotchas_imported = (store.gotchas.len() - before) as u32;
    let _ = store.save(project_root);
}
