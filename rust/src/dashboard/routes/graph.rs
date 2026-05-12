use super::helpers::{detect_project_root_for_dashboard, extract_query_param};

fn project_basename(abs_root: &str) -> String {
    std::path::Path::new(abs_root).file_name().map_or_else(
        || "project".to_string(),
        |n| n.to_string_lossy().to_string(),
    )
}

pub(super) fn handle(
    path: &str,
    query_str: &str,
    _method: &str,
    _body: &str,
) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/heatmap" => {
            let project_root = detect_project_root_for_dashboard();
            let index = crate::core::graph_index::load_or_build(&project_root);
            let entries = build_heatmap_json(&index);
            Some(("200 OK", "application/json", entries))
        }
        "/api/graph" => {
            let root = detect_project_root_for_dashboard();
            let index = crate::core::graph_index::load_or_build(&root);
            let mut val = serde_json::to_value(&index).unwrap_or_default();
            if let Some(obj) = val.as_object_mut() {
                obj.insert(
                    "project_root".to_string(),
                    serde_json::Value::String(project_basename(&root)),
                );
            }
            let json = serde_json::to_string(&val).unwrap_or_else(|_| {
                "{\"error\":\"failed to serialize project index\"}".to_string()
            });
            Some(("200 OK", "application/json", json))
        }
        "/api/graph/enrich" => {
            let root = detect_project_root_for_dashboard();
            let project_path = std::path::Path::new(&root);
            let result = match crate::core::property_graph::CodeGraph::open(project_path) {
                Ok(graph) => {
                    match crate::core::graph_enricher::enrich_graph(&graph, project_path, 500) {
                        Ok(stats) => {
                            let nc = graph.node_count().unwrap_or(0);
                            let ec = graph.edge_count().unwrap_or(0);
                            serde_json::json!({
                                "commits_indexed": stats.commits_indexed,
                                "tests_indexed": stats.tests_indexed,
                                "knowledge_indexed": stats.knowledge_indexed,
                                "edges_created": stats.edges_created,
                                "total_nodes": nc,
                                "total_edges": ec,
                            })
                        }
                        Err(e) => {
                            tracing::warn!("graph enrich error: {e}");
                            serde_json::json!({"error": "enrichment_failed"})
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("graph open error: {e}");
                    serde_json::json!({"error": "graph_unavailable"})
                }
            };
            let json = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/graph/stats" => {
            let root = detect_project_root_for_dashboard();
            let result = if let Some(open) = crate::core::graph_provider::open_best_effort(&root) {
                let nc = open.provider.node_count().unwrap_or(0);
                let ec = open.provider.edge_count().unwrap_or(0);
                match open.source {
                    crate::core::graph_provider::GraphProviderSource::PropertyGraph => {
                        serde_json::json!({
                            "source": "property_graph",
                            "node_count": nc,
                            "edge_count": ec,
                        })
                    }
                    crate::core::graph_provider::GraphProviderSource::GraphIndex => {
                        serde_json::json!({
                            "source": "graph_index",
                            "node_count": nc,
                            "edge_count": ec,
                        })
                    }
                }
            } else {
                serde_json::json!({
                    "source": "none",
                    "node_count": 0,
                    "edge_count": 0,
                })
            };
            let json = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/call-graph" => {
            let root = detect_project_root_for_dashboard();
            let index = crate::core::graph_index::load_or_build(&root);
            let call_graph = crate::core::call_graph::CallGraph::load_or_build(&root, &index);
            let _ = call_graph.save();
            let payload = serde_json::json!({
                "project_root": project_basename(&call_graph.project_root),
                "edges": call_graph.edges,
                "file_hashes": call_graph.file_hashes,
                "indexed_file_count": index.files.len(),
                "indexed_symbol_count": index.symbols.len(),
                "analyzed_file_count": call_graph.file_hashes.len(),
            });
            let json = serde_json::to_string(&payload)
                .unwrap_or_else(|_| "{\"error\":\"failed to serialize call graph\"}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/symbols" => {
            let root = detect_project_root_for_dashboard();
            let index = crate::core::graph_index::load_or_build(&root);
            let q = extract_query_param(query_str, "q");
            let kind = extract_query_param(query_str, "kind");
            let json = build_symbols_json(&index, q.as_deref(), kind.as_deref());
            Some(("200 OK", "application/json", json))
        }
        "/api/health" => {
            let root = detect_project_root_for_dashboard();
            let result =
                crate::tools::ctx_architecture::handle("health", None, &root, Some("json"));
            Some(("200 OK", "application/json", result))
        }
        "/api/hotspots" => {
            let root = detect_project_root_for_dashboard();
            let result =
                crate::tools::ctx_architecture::handle("hotspots", None, &root, Some("json"));
            Some(("200 OK", "application/json", result))
        }
        "/api/communities" => {
            let root = detect_project_root_for_dashboard();
            let result =
                crate::tools::ctx_architecture::handle("communities", None, &root, Some("json"));
            Some(("200 OK", "application/json", result))
        }
        "/api/smells" => {
            let root = detect_project_root_for_dashboard();
            let rule = extract_query_param(query_str, "rule");
            let path_filter = extract_query_param(query_str, "path");
            let result = crate::tools::ctx_smells::handle(
                "scan",
                rule.as_deref(),
                path_filter.as_deref(),
                &root,
                Some("json"),
            );
            Some(("200 OK", "application/json", result))
        }
        "/api/smells/summary" => {
            let root = detect_project_root_for_dashboard();
            let result =
                crate::tools::ctx_smells::handle("summary", None, None, &root, Some("json"));
            Some(("200 OK", "application/json", result))
        }
        "/api/routes" => {
            let root = detect_project_root_for_dashboard();
            let index = crate::core::graph_index::load_or_build(&root);
            let routes =
                crate::core::route_extractor::extract_routes_from_project(&root, &index.files);
            let route_candidate_count = index
                .files
                .keys()
                .filter(|p| {
                    std::path::Path::new(p.as_str())
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| {
                            matches!(e, "js" | "ts" | "py" | "rs" | "java" | "rb" | "go" | "kt")
                        })
                })
                .count();
            let payload = serde_json::json!({
                "routes": routes,
                "indexed_file_count": index.files.len(),
                "route_candidate_count": route_candidate_count,
            });
            let json =
                serde_json::to_string(&payload).unwrap_or_else(|_| "{\"routes\":[]}".to_string());
            Some(("200 OK", "application/json", json))
        }
        _ => None,
    }
}

fn build_symbols_json(
    index: &crate::core::graph_index::ProjectIndex,
    query: Option<&str>,
    kind: Option<&str>,
) -> String {
    let query = query
        .map(|q| q.trim().to_lowercase())
        .filter(|q| !q.is_empty());
    let kind = kind
        .map(|k| k.trim().to_lowercase())
        .filter(|k| !k.is_empty());

    let mut symbols: Vec<&crate::core::graph_index::SymbolEntry> = index
        .symbols
        .values()
        .filter(|sym| {
            let kind_match = match kind.as_ref() {
                Some(k) => sym.kind.eq_ignore_ascii_case(k),
                None => true,
            };
            let query_match = match query.as_ref() {
                Some(q) => {
                    let name = sym.name.to_lowercase();
                    let file = sym.file.to_lowercase();
                    let symbol_kind = sym.kind.to_lowercase();
                    name.contains(q) || file.contains(q) || symbol_kind.contains(q)
                }
                None => true,
            };
            kind_match && query_match
        })
        .collect();

    symbols.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.start_line.cmp(&b.start_line))
            .then_with(|| a.name.cmp(&b.name))
    });
    symbols.truncate(500);

    serde_json::to_string(
        &symbols
            .into_iter()
            .map(|sym| {
                serde_json::json!({
                    "name": sym.name,
                    "kind": sym.kind,
                    "file": sym.file,
                    "start_line": sym.start_line,
                    "end_line": sym.end_line,
                    "is_exported": sym.is_exported,
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string())
}

fn build_heatmap_json(index: &crate::core::graph_index::ProjectIndex) -> String {
    let mut connection_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for edge in &index.edges {
        *connection_counts.entry(edge.from.clone()).or_default() += 1;
        *connection_counts.entry(edge.to.clone()).or_default() += 1;
    }

    let max_tokens = index
        .files
        .values()
        .map(|f| f.token_count)
        .max()
        .unwrap_or(1) as f64;
    let max_connections = connection_counts.values().max().copied().unwrap_or(1) as f64;

    let mut entries: Vec<serde_json::Value> = index
        .files
        .values()
        .map(|f| {
            let connections = connection_counts.get(&f.path).copied().unwrap_or(0);
            let token_norm = f.token_count as f64 / max_tokens;
            let conn_norm = connections as f64 / max_connections;
            let heat = token_norm * 0.4 + conn_norm * 0.6;
            serde_json::json!({
                "path": f.path,
                "tokens": f.token_count,
                "connections": connections,
                "language": f.language,
                "heat": (heat * 100.0).round() / 100.0,
            })
        })
        .collect();

    entries.sort_by(|a, b| {
        b["heat"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["heat"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
}
