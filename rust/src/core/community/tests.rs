use std::collections::HashSet;

use super::graph::AdjGraph;
use super::{analyze, detect_communities, hardening, leiden, stable_ids};
use crate::core::property_graph::{CodeGraph, Edge, EdgeKind, Node};

// ── PropertyGraph-backed tests (end-to-end through SQLite) ──────────────

fn build_test_graph() -> CodeGraph {
    let graph = CodeGraph::open_in_memory().unwrap();

    let na = graph.upsert_node(&Node::file("src/core/a.rs")).unwrap();
    let nb = graph.upsert_node(&Node::file("src/core/b.rs")).unwrap();
    let nc = graph.upsert_node(&Node::file("src/core/c.rs")).unwrap();
    let nd = graph.upsert_node(&Node::file("src/tools/d.rs")).unwrap();
    let ne = graph.upsert_node(&Node::file("src/tools/e.rs")).unwrap();

    graph
        .upsert_edge(&Edge::new(na, nb, EdgeKind::Imports))
        .unwrap();
    graph
        .upsert_edge(&Edge::new(nb, nc, EdgeKind::Imports))
        .unwrap();
    graph
        .upsert_edge(&Edge::new(na, nc, EdgeKind::Calls))
        .unwrap();
    graph
        .upsert_edge(&Edge::new(nd, ne, EdgeKind::Imports))
        .unwrap();
    graph
        .upsert_edge(&Edge::new(ne, nd, EdgeKind::Calls))
        .unwrap();
    graph
        .upsert_edge(&Edge::new(nc, nd, EdgeKind::Imports))
        .unwrap();

    graph
}

#[test]
fn detects_communities() {
    let g = build_test_graph();
    let result = detect_communities(g.connection());
    assert!(!result.communities.is_empty());
    assert_eq!(result.node_count, 5);
    assert!(result.edge_count > 0);
}

#[test]
fn modularity_non_negative() {
    let g = build_test_graph();
    let result = detect_communities(g.connection());
    assert!(result.modularity >= 0.0);
}

#[test]
fn community_files_cover_all_nodes() {
    let g = build_test_graph();
    let result = detect_communities(g.connection());
    let total_files: usize = result.communities.iter().map(|c| c.files.len()).sum();
    assert_eq!(total_files, 5);
}

#[test]
fn empty_graph() {
    let g = CodeGraph::open_in_memory().unwrap();
    let result = detect_communities(g.connection());
    assert!(result.communities.is_empty());
    assert_eq!(result.modularity, 0.0);
    assert_eq!(result.node_count, 0);
}

#[test]
fn communities_are_connected() {
    let g = build_test_graph();
    let graph = AdjGraph::from_property_graph(g.connection());
    let result = detect_communities(g.connection());

    for comm in &result.communities {
        if comm.files.len() <= 1 {
            continue;
        }
        let indices: Vec<usize> = comm
            .files
            .iter()
            .filter_map(|f| graph.node_to_idx.get(f).copied())
            .collect();
        let components = leiden::find_connected_components(&graph, &indices);
        assert_eq!(
            components.len(),
            1,
            "community {} should be connected",
            comm.id
        );
    }
}

// ── Engine tests on synthetic topologies ───────────────────────────────

fn names(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("f{i}")).collect()
}

/// Two triangles joined by a single bridge edge.
fn two_triangles(n: usize) -> AdjGraph {
    let mut edges = vec![
        (0, 1, "imports"),
        (1, 2, "imports"),
        (0, 2, "imports"),
        (3, 4, "imports"),
        (4, 5, "imports"),
        (3, 5, "imports"),
        (2, 3, "imports"), // bridge
    ];
    if n > 6 {
        edges.push((5, 6, "imports")); // grow cluster B
    }
    AdjGraph::from_test_edges(names(n), &edges)
}

#[test]
fn partition_is_deterministic() {
    let g = two_triangles(6);
    let (a1, r1) = analyze(&g, None);
    let (a2, r2) = analyze(&g, None);
    assert_eq!(a1, a2, "assignment must be identical across runs");
    assert_eq!(r1.assignment(), r2.assignment());
    assert!(r1.communities.len() >= 2, "triangles must not be merged");
}

#[test]
fn ids_stable_across_rebuild() {
    // Build 1: two equal triangles.
    let g1 = two_triangles(6);
    let (_, r1) = analyze(&g1, None);
    let map1 = r1.assignment();

    // Build 2: cluster B grew by one node, flipping size order. Without remap the
    // larger cluster would steal id 0; remap must keep both ids stable.
    let g2 = two_triangles(7);
    let (_, r2) = analyze(&g2, Some(&map1));
    let map2 = r2.assignment();

    assert_eq!(
        map1["f0"], map2["f0"],
        "cluster A keeps its id across rebuild"
    );
    assert_eq!(
        map1["f3"], map2["f3"],
        "cluster B keeps its id even though it grew"
    );
    assert_eq!(map2["f3"], map2["f6"], "new node joins cluster B");
}

/// Two 8-node clusters bridged only through a single high-degree hub.
fn hub_bridged_graph() -> AdjGraph {
    let mut edges: Vec<(usize, usize, &str)> = Vec::new();
    // Cluster A: ring 0..7 + chords.
    for i in 0..8 {
        edges.push((i, (i + 1) % 8, "imports"));
    }
    edges.push((0, 2, "imports"));
    edges.push((4, 6, "imports"));
    // Cluster B: ring 8..15 + chords.
    for i in 8..16 {
        let next = if i == 15 { 8 } else { i + 1 };
        edges.push((i, next, "imports"));
    }
    edges.push((8, 10, "imports"));
    edges.push((12, 14, "imports"));
    // Hub 16 connects to four nodes in each cluster.
    for &t in &[0, 1, 2, 3, 8, 9, 10, 11] {
        edges.push((16, t, "imports"));
    }
    AdjGraph::from_test_edges(names(17), &edges)
}

#[test]
fn hub_exclusion_prevents_collapse() {
    let g = hub_bridged_graph();
    let (assignment, result) = analyze(&g, None);

    // Every node is assigned (no sentinel leftover from hub handling).
    assert!(assignment.iter().all(|&c| c != usize::MAX));
    let covered: usize = result.communities.iter().map(|c| c.files.len()).sum();
    assert_eq!(covered, 17);

    let map = result.assignment();
    assert!(
        result.communities.len() >= 2,
        "hub must not collapse the two clusters into one"
    );
    assert_ne!(
        map["f0"], map["f8"],
        "cluster A and B stay in different communities"
    );
}

#[test]
fn resplit_breaks_oversized_community() {
    // Barbell: two 6-cliques joined by one edge.
    let mut edges: Vec<(usize, usize, &str)> = Vec::new();
    for grp in 0..2 {
        let base = grp * 6;
        for i in base..base + 6 {
            for j in (i + 1)..base + 6 {
                edges.push((i, j, "imports"));
            }
        }
    }
    edges.push((5, 6, "imports")); // bridge
    let g = AdjGraph::from_test_edges(names(12), &edges);

    // Force everything into one community, then let the resplit pass run.
    let mut assignment = vec![0usize; 12];
    hardening::split_oversized_and_incohesive(&g, &mut assignment);
    let distinct: HashSet<usize> = assignment.iter().copied().collect();
    assert!(
        distinct.len() >= 2,
        "oversized community should be split, got {} communities",
        distinct.len()
    );
}

#[test]
fn remap_assigns_fresh_ids_without_overlap() {
    // A graph whose nodes share no names with `prev` → all fresh ids, no panic.
    let g = two_triangles(6);
    let (canonical, _) = analyze(&g, None);
    let mut prev = std::collections::HashMap::new();
    prev.insert("unrelated_a".to_string(), 0usize);
    prev.insert("unrelated_b".to_string(), 1usize);
    let remapped = stable_ids::remap_to_previous(&g, &canonical, &prev);
    assert_eq!(remapped.len(), canonical.len());
    // Same partition structure (same number of distinct communities).
    let before: HashSet<usize> = canonical.iter().copied().collect();
    let after: HashSet<usize> = remapped.iter().copied().collect();
    assert_eq!(before.len(), after.len());
}
