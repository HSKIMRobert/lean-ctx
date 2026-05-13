use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::deep_queries;
use super::graph_index::{normalize_project_root, ProjectIndex, SymbolEntry};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    pub project_root: String,
    pub edges: Vec<CallEdge>,
    pub file_hashes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub caller_file: String,
    pub caller_symbol: String,
    pub caller_line: usize,
    pub callee_name: String,
}

// ---------------------------------------------------------------------------
// Background build state (singleton per process)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct BuildProgress {
    pub status: &'static str,
    pub files_total: usize,
    pub files_done: usize,
    pub edges_found: usize,
}

enum BuildState {
    Idle,
    Building {
        files_total: usize,
        files_done: Arc<AtomicUsize>,
        edges_found: Arc<AtomicUsize>,
    },
    Ready(Arc<CallGraph>),
    Failed(String),
}

static BUILD: OnceLock<Mutex<BuildState>> = OnceLock::new();

fn global_state() -> &'static Mutex<BuildState> {
    BUILD.get_or_init(|| Mutex::new(BuildState::Idle))
}

impl CallGraph {
    pub fn new(project_root: &str) -> Self {
        Self {
            project_root: normalize_project_root(project_root),
            edges: Vec::new(),
            file_hashes: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Parallel build — processes files via rayon thread pool
    // -----------------------------------------------------------------------

    pub fn build_parallel(
        index: &ProjectIndex,
        progress: Option<(&AtomicUsize, &AtomicUsize)>,
    ) -> Self {
        let project_root = &index.project_root;
        let symbols_by_file = group_symbols_by_file_owned(index);
        let file_keys: Vec<String> = index.files.keys().cloned().collect();

        let results: Vec<(String, String, Vec<CallEdge>)> = file_keys
            .par_iter()
            .filter_map(|rel_path| {
                let abs_path = resolve_path(rel_path, project_root);
                let content = std::fs::read_to_string(&abs_path).ok()?;
                let hash = simple_hash(&content);

                let ext = Path::new(rel_path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");

                let analysis = deep_queries::analyze(&content, ext);
                let file_symbols = symbols_by_file.get(rel_path.as_str());

                let edges: Vec<CallEdge> = analysis
                    .calls
                    .iter()
                    .map(|call| {
                        let caller_sym = find_enclosing_symbol_owned(file_symbols, call.line + 1);
                        CallEdge {
                            caller_file: rel_path.clone(),
                            caller_symbol: caller_sym,
                            caller_line: call.line + 1,
                            callee_name: call.callee.clone(),
                        }
                    })
                    .collect();

                if let Some((done, edge_count)) = progress {
                    done.fetch_add(1, Ordering::Relaxed);
                    edge_count.fetch_add(edges.len(), Ordering::Relaxed);
                }

                Some((rel_path.clone(), hash, edges))
            })
            .collect();

        let mut graph = Self::new(project_root);
        let edge_capacity: usize = results.iter().map(|(_, _, e)| e.len()).sum();
        graph.edges.reserve(edge_capacity);
        graph.file_hashes.reserve(results.len());

        for (path, hash, edges) in results {
            graph.file_hashes.insert(path, hash);
            graph.edges.extend(edges);
        }

        graph
    }

    // -----------------------------------------------------------------------
    // Incremental parallel build — only re-analyzes changed files
    // -----------------------------------------------------------------------

    pub fn build_incremental_parallel(
        index: &ProjectIndex,
        previous: &CallGraph,
        progress: Option<(&AtomicUsize, &AtomicUsize)>,
    ) -> Self {
        let project_root = &index.project_root;
        let symbols_by_file = group_symbols_by_file_owned(index);
        let file_keys: Vec<String> = index.files.keys().cloned().collect();

        let prev_edges_by_file = group_edges_by_file(&previous.edges);

        let results: Vec<(String, String, Vec<CallEdge>)> = file_keys
            .par_iter()
            .filter_map(|rel_path| {
                let abs_path = resolve_path(rel_path, project_root);
                let content = std::fs::read_to_string(&abs_path).ok()?;
                let hash = simple_hash(&content);
                let changed = previous.file_hashes.get(rel_path.as_str()) != Some(&hash);

                let edges = if changed {
                    let ext = Path::new(rel_path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");

                    let analysis = deep_queries::analyze(&content, ext);
                    let file_symbols = symbols_by_file.get(rel_path.as_str());

                    analysis
                        .calls
                        .iter()
                        .map(|call| {
                            let caller_sym =
                                find_enclosing_symbol_owned(file_symbols, call.line + 1);
                            CallEdge {
                                caller_file: rel_path.clone(),
                                caller_symbol: caller_sym,
                                caller_line: call.line + 1,
                                callee_name: call.callee.clone(),
                            }
                        })
                        .collect()
                } else {
                    prev_edges_by_file
                        .get(rel_path.as_str())
                        .cloned()
                        .unwrap_or_default()
                };

                if let Some((done, edge_count)) = progress {
                    done.fetch_add(1, Ordering::Relaxed);
                    edge_count.fetch_add(edges.len(), Ordering::Relaxed);
                }

                Some((rel_path.clone(), hash, edges))
            })
            .collect();

        let mut graph = Self::new(project_root);
        let edge_capacity: usize = results.iter().map(|(_, _, e)| e.len()).sum();
        graph.edges.reserve(edge_capacity);
        graph.file_hashes.reserve(results.len());

        for (path, hash, edges) in results {
            graph.file_hashes.insert(path, hash);
            graph.edges.extend(edges);
        }

        graph
    }

    // -----------------------------------------------------------------------
    // Public API: non-blocking access for the dashboard
    // -----------------------------------------------------------------------

    /// Returns the cached graph immediately, or `None` + starts a background build.
    pub fn get_or_start_build(
        project_root: &str,
        index: Arc<ProjectIndex>,
    ) -> Result<Arc<CallGraph>, BuildProgress> {
        let state = global_state();
        let mut guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match &*guard {
            BuildState::Ready(graph) => return Ok(Arc::clone(graph)),
            BuildState::Building {
                files_total,
                files_done,
                edges_found,
            } => {
                return Err(BuildProgress {
                    status: "building",
                    files_total: *files_total,
                    files_done: files_done.load(Ordering::Relaxed),
                    edges_found: edges_found.load(Ordering::Relaxed),
                });
            }
            BuildState::Failed(msg) => {
                tracing::warn!("[call_graph: previous build failed: {msg} — retrying]");
            }
            BuildState::Idle => {}
        }

        // Try serving from disk cache first
        if let Some(cached) = Self::load(project_root) {
            if !cache_looks_stale(&cached, &index) {
                let arc = Arc::new(cached);
                *guard = BuildState::Ready(Arc::clone(&arc));
                return Ok(arc);
            }
        }

        let files_total = index.files.len();
        let files_done = Arc::new(AtomicUsize::new(0));
        let edges_found = Arc::new(AtomicUsize::new(0));

        *guard = BuildState::Building {
            files_total,
            files_done: Arc::clone(&files_done),
            edges_found: Arc::clone(&edges_found),
        };
        drop(guard);

        let root = normalize_project_root(project_root);
        let fd = Arc::clone(&files_done);
        let ef = Arc::clone(&edges_found);

        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let previous = CallGraph::load(&root);
                if let Some(prev) = &previous {
                    CallGraph::build_incremental_parallel(&index, prev, Some((&fd, &ef)))
                } else {
                    CallGraph::build_parallel(&index, Some((&fd, &ef)))
                }
            }));

            match result {
                Ok(graph) => {
                    let _ = graph.save();
                    let arc = Arc::new(graph);
                    if let Ok(mut g) = global_state().lock() {
                        *g = BuildState::Ready(Arc::clone(&arc));
                    }
                    tracing::info!(
                        "[call_graph: build complete — {} files, {} edges]",
                        arc.file_hashes.len(),
                        arc.edges.len()
                    );
                }
                Err(e) => {
                    let msg = format!("{e:?}");
                    tracing::error!("[call_graph: build panicked: {msg}]");
                    if let Ok(mut g) = global_state().lock() {
                        *g = BuildState::Failed(msg);
                    }
                }
            }
        });

        Err(BuildProgress {
            status: "building",
            files_total,
            files_done: 0,
            edges_found: 0,
        })
    }

    /// Returns current build status without starting anything.
    pub fn build_status() -> BuildProgress {
        let state = global_state();
        let guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match &*guard {
            BuildState::Idle => BuildProgress {
                status: "idle",
                files_total: 0,
                files_done: 0,
                edges_found: 0,
            },
            BuildState::Building {
                files_total,
                files_done,
                edges_found,
            } => BuildProgress {
                status: "building",
                files_total: *files_total,
                files_done: files_done.load(Ordering::Relaxed),
                edges_found: edges_found.load(Ordering::Relaxed),
            },
            BuildState::Ready(graph) => BuildProgress {
                status: "ready",
                files_total: graph.file_hashes.len(),
                files_done: graph.file_hashes.len(),
                edges_found: graph.edges.len(),
            },
            BuildState::Failed(msg) => {
                tracing::debug!("[call_graph: status check — failed: {msg}]");
                BuildProgress {
                    status: "error",
                    files_total: 0,
                    files_done: 0,
                    edges_found: 0,
                }
            }
        }
    }

    /// Force-invalidate the cached result so next request triggers a rebuild.
    pub fn invalidate() {
        if let Ok(mut g) = global_state().lock() {
            *g = BuildState::Idle;
        }
    }

    // -----------------------------------------------------------------------
    // Legacy synchronous API (kept for non-dashboard callers)
    // -----------------------------------------------------------------------

    pub fn build(index: &ProjectIndex) -> Self {
        Self::build_parallel(index, None)
    }

    pub fn build_incremental(index: &ProjectIndex, previous: &CallGraph) -> Self {
        Self::build_incremental_parallel(index, previous, None)
    }

    pub fn callers_of(&self, symbol: &str) -> Vec<&CallEdge> {
        let sym_lower = symbol.to_lowercase();
        self.edges
            .iter()
            .filter(|e| e.callee_name.to_lowercase() == sym_lower)
            .collect()
    }

    pub fn callees_of(&self, symbol: &str) -> Vec<&CallEdge> {
        let sym_lower = symbol.to_lowercase();
        self.edges
            .iter()
            .filter(|e| e.caller_symbol.to_lowercase() == sym_lower)
            .collect()
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = call_graph_dir(&self.project_root)
            .ok_or_else(|| "Cannot determine home directory".to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("call_graph.json"), json).map_err(|e| e.to_string())
    }

    pub fn load(project_root: &str) -> Option<Self> {
        let dir = call_graph_dir(project_root)?;
        let path = dir.join("call_graph.json");
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn load_or_build(project_root: &str, index: &ProjectIndex) -> Self {
        if let Some(previous) = Self::load(project_root) {
            Self::build_incremental(index, &previous)
        } else {
            Self::build(index)
        }
    }
}

// ---------------------------------------------------------------------------
// Cache staleness check (fast — mtime-based, no content reads)
// ---------------------------------------------------------------------------

fn cache_looks_stale(cached: &CallGraph, index: &ProjectIndex) -> bool {
    if cached.file_hashes.len() != index.files.len() {
        return true;
    }
    let cached_files: std::collections::HashSet<&str> =
        cached.file_hashes.keys().map(String::as_str).collect();
    let index_files: std::collections::HashSet<&str> =
        index.files.keys().map(String::as_str).collect();
    cached_files != index_files
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn call_graph_dir(project_root: &str) -> Option<std::path::PathBuf> {
    ProjectIndex::index_dir(project_root)
}

fn group_edges_by_file(edges: &[CallEdge]) -> HashMap<&str, Vec<CallEdge>> {
    let mut map: HashMap<&str, Vec<CallEdge>> = HashMap::new();
    for edge in edges {
        map.entry(edge.caller_file.as_str())
            .or_default()
            .push(edge.clone());
    }
    map
}

/// Owned version for safe `Send` across rayon threads.
fn group_symbols_by_file_owned(index: &ProjectIndex) -> HashMap<String, Vec<SymbolEntry>> {
    let mut map: HashMap<String, Vec<SymbolEntry>> = HashMap::new();
    for sym in index.symbols.values() {
        map.entry(sym.file.clone()).or_default().push(sym.clone());
    }
    for syms in map.values_mut() {
        syms.sort_by_key(|s| s.start_line);
    }
    map
}

fn find_enclosing_symbol_owned(file_symbols: Option<&Vec<SymbolEntry>>, line: usize) -> String {
    let Some(syms) = file_symbols else {
        return "<module>".to_string();
    };
    let mut best: Option<&SymbolEntry> = None;
    for sym in syms {
        if line >= sym.start_line && line <= sym.end_line {
            match best {
                None => best = Some(sym),
                Some(prev) => {
                    if (sym.end_line - sym.start_line) < (prev.end_line - prev.start_line) {
                        best = Some(sym);
                    }
                }
            }
        }
    }
    best.map_or_else(|| "<module>".to_string(), |s| s.name.clone())
}

fn resolve_path(relative: &str, project_root: &str) -> String {
    let p = Path::new(relative);
    if p.is_absolute() && p.exists() {
        return relative.to_string();
    }
    let relative = relative.trim_start_matches(['/', '\\']);
    let joined = Path::new(project_root).join(relative);
    joined.to_string_lossy().to_string()
}

fn simple_hash(content: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callers_of_empty_graph() {
        let graph = CallGraph::new("/tmp");
        assert!(graph.callers_of("foo").is_empty());
    }

    #[test]
    fn callers_of_finds_edges() {
        let mut graph = CallGraph::new("/tmp");
        graph.edges.push(CallEdge {
            caller_file: "a.rs".to_string(),
            caller_symbol: "bar".to_string(),
            caller_line: 10,
            callee_name: "foo".to_string(),
        });
        graph.edges.push(CallEdge {
            caller_file: "b.rs".to_string(),
            caller_symbol: "baz".to_string(),
            caller_line: 20,
            callee_name: "foo".to_string(),
        });
        graph.edges.push(CallEdge {
            caller_file: "c.rs".to_string(),
            caller_symbol: "qux".to_string(),
            caller_line: 30,
            callee_name: "other".to_string(),
        });
        let callers = graph.callers_of("foo");
        assert_eq!(callers.len(), 2);
    }

    #[test]
    fn callees_of_finds_edges() {
        let mut graph = CallGraph::new("/tmp");
        graph.edges.push(CallEdge {
            caller_file: "a.rs".to_string(),
            caller_symbol: "main".to_string(),
            caller_line: 5,
            callee_name: "init".to_string(),
        });
        graph.edges.push(CallEdge {
            caller_file: "a.rs".to_string(),
            caller_symbol: "main".to_string(),
            caller_line: 6,
            callee_name: "run".to_string(),
        });
        graph.edges.push(CallEdge {
            caller_file: "a.rs".to_string(),
            caller_symbol: "other".to_string(),
            caller_line: 15,
            callee_name: "init".to_string(),
        });
        let callees = graph.callees_of("main");
        assert_eq!(callees.len(), 2);
    }

    #[test]
    fn find_enclosing_picks_narrowest() {
        let outer = SymbolEntry {
            file: "a.rs".to_string(),
            name: "Outer".to_string(),
            kind: "struct".to_string(),
            start_line: 1,
            end_line: 50,
            is_exported: true,
        };
        let inner = SymbolEntry {
            file: "a.rs".to_string(),
            name: "inner_fn".to_string(),
            kind: "fn".to_string(),
            start_line: 10,
            end_line: 20,
            is_exported: false,
        };
        let syms = vec![outer, inner];
        let result = find_enclosing_symbol_owned(Some(&syms), 15);
        assert_eq!(result, "inner_fn");
    }

    #[test]
    fn find_enclosing_returns_module_when_no_match() {
        let sym = SymbolEntry {
            file: "a.rs".to_string(),
            name: "foo".to_string(),
            kind: "fn".to_string(),
            start_line: 10,
            end_line: 20,
            is_exported: false,
        };
        let syms = vec![sym];
        let result = find_enclosing_symbol_owned(Some(&syms), 5);
        assert_eq!(result, "<module>");
    }

    #[test]
    fn resolve_path_trims_rooted_relative_prefix() {
        let resolved = resolve_path(r"\src\main\kotlin\Example.kt", r"C:\repo");
        assert_eq!(
            resolved,
            Path::new(r"C:\repo")
                .join(r"src\main\kotlin\Example.kt")
                .to_string_lossy()
                .to_string()
        );
    }
}
