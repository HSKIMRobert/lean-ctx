use std::collections::{HashSet, VecDeque};

use super::partial::{EdgeKind, NodeInfo, PartialGraph};

/// A neighbor node discovered during expansion.
#[derive(Debug, Clone)]
pub struct NeighborNode {
    /// Symbol name.
    pub name: String,
    /// File containing the symbol.
    pub file: String,
    /// Symbol kind, such as function, method, or type.
    pub kind: String,
    /// Hop distance from the expansion center.
    pub depth: usize,
    /// Relationship from the node that discovered this neighbor.
    pub relation: EdgeKind,
}

/// Expand `k` hops from a center symbol using the provided edge lookup.
///
/// The edge lookup returns neighbor name, file, kind, and relation for a symbol.
pub(crate) fn expand_neighborhood<F>(
    center: &str,
    center_file: &str,
    center_kind: &str,
    k: usize,
    edge_fn: F,
) -> PartialGraph
where
    F: Fn(&str) -> Vec<(String, String, String, EdgeKind)>,
{
    let mut graph = PartialGraph {
        center: center.to_string(),
        max_depth: k,
        ..PartialGraph::default()
    };
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    graph.nodes.insert(
        center.to_string(),
        NodeInfo {
            file: center_file.to_string(),
            kind: center_kind.to_string(),
            depth: 0,
        },
    );
    visited.insert(center.to_string());
    queue.push_back((center.to_string(), 0));

    while let Some((current, depth)) = queue.pop_front() {
        if depth >= k {
            continue;
        }

        for (neighbor, file, kind, relation) in edge_fn(&current) {
            if visited.insert(neighbor.clone()) {
                graph.nodes.insert(
                    neighbor.clone(),
                    NodeInfo {
                        file,
                        kind,
                        depth: depth + 1,
                    },
                );
                queue.push_back((neighbor.clone(), depth + 1));
            }
            graph.edges.push((current.clone(), neighbor, relation));
        }
    }

    graph
}

#[cfg(test)]
mod tests {
    use super::{EdgeKind, expand_neighborhood};

    fn test_edges(symbol: &str) -> Vec<(String, String, String, EdgeKind)> {
        let neighbors: &[(&str, &str, EdgeKind)] = match symbol {
            "root" => &[
                ("direct", "src/direct.rs", EdgeKind::Calls),
                ("caller", "src/caller.rs", EdgeKind::CalledBy),
            ],
            "direct" => &[("deep", "src/deep.rs", EdgeKind::Calls)],
            "deep" => &[("root", "src/root.rs", EdgeKind::Calls)],
            _ => &[],
        };
        neighbors
            .iter()
            .map(|(name, file, relation)| {
                (
                    (*name).to_string(),
                    (*file).to_string(),
                    "function".to_string(),
                    *relation,
                )
            })
            .collect()
    }

    #[test]
    fn expand_zero_hops_returns_center_only() {
        let graph = expand_neighborhood("root", "src/root.rs", "function", 0, test_edges);

        assert_eq!(graph.node_count(), 1);
        assert!(graph.edges.is_empty());
        assert_eq!(graph.at_depth(0), vec!["root"]);
    }

    #[test]
    fn expand_one_hop_finds_direct_neighbors() {
        let graph = expand_neighborhood("root", "src/root.rs", "function", 1, test_edges);

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.at_depth(1), vec!["caller", "direct"]);
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn expand_two_hops_finds_transitive_neighbors() {
        let graph = expand_neighborhood("root", "src/root.rs", "function", 2, test_edges);

        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.at_depth(2), vec!["deep"]);
        assert_eq!(graph.max_depth, 2);
    }

    #[test]
    fn cycles_do_not_cause_infinite_loop() {
        let graph = expand_neighborhood("root", "src/root.rs", "function", 3, test_edges);

        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.at_depth(0), vec!["root"]);
        assert_eq!(graph.edges.len(), 4);
    }

    #[test]
    fn expand_with_no_edges_returns_center() {
        let graph = expand_neighborhood("isolated", "src/lib.rs", "function", 4, |_| vec![]);

        assert_eq!(graph.node_count(), 1);
        assert!(graph.edges.is_empty());
    }
}
