//! Partial call-graph representation for bounded neighborhood expansion.
//!
//! Stores nodes, directed edges, and hop depth from a center symbol for
//! compact LLM context injection.

use std::collections::HashMap;

/// A partial subgraph centered on a target symbol.
#[derive(Debug, Clone, Default)]
pub struct PartialGraph {
    /// Nodes in the subgraph, keyed by symbol name.
    pub nodes: HashMap<String, NodeInfo>,
    /// Directed edges represented as source, target, and relation type.
    pub edges: Vec<(String, String, EdgeKind)>,
    /// The center symbol this graph was expanded from.
    pub center: String,
    /// Maximum depth requested during expansion.
    pub max_depth: usize,
}

/// Metadata for a symbol included in a partial graph.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// File containing the symbol.
    pub file: String,
    /// Symbol kind, such as function, method, or type.
    pub kind: String,
    /// Shortest hop distance from the center symbol.
    pub depth: usize,
}

/// Relationship represented by a directed graph edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// The source symbol calls the target symbol.
    Calls,
    /// The source symbol is called by the target symbol.
    CalledBy,
    /// The source symbol imports the target symbol.
    Imports,
    /// The source symbol implements the target symbol.
    Implements,
}

impl PartialGraph {
    /// Render the subgraph as deterministic compact text for LLM context injection.
    pub fn render(&self) -> String {
        let Some(center) = self.nodes.get(&self.center) else {
            return String::new();
        };

        let mut output = format!("Center: {} ({})", self.center, center.file);
        for depth in 1..=self.max_depth {
            let mut nodes: Vec<(&str, &NodeInfo)> = self
                .nodes
                .iter()
                .filter(|(_, info)| info.depth == depth)
                .map(|(name, info)| (name.as_str(), info))
                .collect();
            nodes.sort_unstable_by_key(|(name, _)| *name);

            if nodes.is_empty() {
                continue;
            }

            output.push_str(&format!("\nDepth {depth}: "));
            let rendered = nodes
                .into_iter()
                .map(|(name, info)| {
                    let direction = self.direction_for(name, depth);
                    format!("{direction}{name} ({})", info.file)
                })
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str(&rendered);
        }
        output
    }

    /// Return the count of nodes in the subgraph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Return symbols at a specific depth in deterministic name order.
    pub fn at_depth(&self, depth: usize) -> Vec<&str> {
        let mut symbols: Vec<&str> = self
            .nodes
            .iter()
            .filter(|(_, info)| info.depth == depth)
            .map(|(name, _)| name.as_str())
            .collect();
        symbols.sort_unstable();
        symbols
    }

    fn direction_for(&self, name: &str, depth: usize) -> String {
        let relation = self.edges.iter().find_map(|(from, to, relation)| {
            if to == name {
                self.nodes
                    .get(from)
                    .filter(|info| info.depth + 1 == depth)
                    .map(|_| *relation)
            } else {
                None
            }
        });
        let arrow = match relation {
            Some(EdgeKind::CalledBy) => "← ",
            Some(EdgeKind::Calls | EdgeKind::Imports | EdgeKind::Implements) | None => "→ ",
        };
        arrow.repeat(depth)
    }
}

#[cfg(test)]
mod tests {
    use super::{NodeInfo, PartialGraph};

    #[test]
    fn empty_graph_renders_empty() {
        assert_eq!(PartialGraph::default().render(), "");
    }

    #[test]
    fn single_node_graph() {
        let mut graph = PartialGraph {
            center: "root".to_string(),
            max_depth: 2,
            ..PartialGraph::default()
        };
        graph.nodes.insert(
            "root".to_string(),
            NodeInfo {
                file: "src/root.rs".to_string(),
                kind: "function".to_string(),
                depth: 0,
            },
        );

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.render(), "Center: root (src/root.rs)");
    }

    #[test]
    fn at_depth_filters_correctly() {
        let mut graph = PartialGraph::default();
        for (name, depth) in [("root", 0), ("beta", 1), ("alpha", 1), ("leaf", 2)] {
            graph.nodes.insert(
                name.to_string(),
                NodeInfo {
                    file: format!("{name}.rs"),
                    kind: "function".to_string(),
                    depth,
                },
            );
        }

        assert_eq!(graph.at_depth(1), vec!["alpha", "beta"]);
        assert_eq!(graph.at_depth(2), vec!["leaf"]);
        assert!(graph.at_depth(3).is_empty());
    }
}
