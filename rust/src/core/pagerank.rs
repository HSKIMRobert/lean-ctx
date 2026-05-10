//! PageRank computation on the Property Graph.
//!
//! Provides a reusable `compute` function that can be called by
//! `ctx_architecture`, `ctx_overview`, and `ctx_fill` for importance-weighted
//! context selection.

use std::collections::{HashMap, HashSet};

use rusqlite::Connection;

pub struct PageRankInput {
    pub files: HashSet<String>,
    pub forward: HashMap<String, Vec<String>>,
}

impl PageRankInput {
    pub fn from_connection(conn: &Connection) -> Self {
        let mut files: HashSet<String> = HashSet::new();
        let mut forward: HashMap<String, Vec<String>> = HashMap::new();

        if let Ok(mut stmt) =
            conn.prepare("SELECT DISTINCT file_path FROM nodes WHERE kind = 'file'")
        {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for f in rows.flatten() {
                    files.insert(f);
                }
            }
        }

        let edge_sql = "
            SELECT DISTINCT n1.file_path, n2.file_path
            FROM edges e
            JOIN nodes n1 ON e.source_id = n1.id
            JOIN nodes n2 ON e.target_id = n2.id
            WHERE n1.kind = 'file' AND n2.kind = 'file'
              AND n1.file_path != n2.file_path
        ";
        if let Ok(mut stmt) = conn.prepare(edge_sql) {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                for row in rows.flatten() {
                    let (src, tgt) = row;
                    forward.entry(src).or_default().push(tgt);
                }
            }
        }

        for deps in forward.values_mut() {
            deps.sort();
            deps.dedup();
        }

        Self { files, forward }
    }
}

pub fn compute(input: &PageRankInput, damping: f64, iterations: usize) -> HashMap<String, f64> {
    let n = input.files.len();
    if n == 0 {
        return HashMap::new();
    }

    let init = 1.0 / n as f64;
    let mut rank: HashMap<String, f64> = input.files.iter().map(|f| (f.clone(), init)).collect();

    for _ in 0..iterations {
        let mut new_rank: HashMap<String, f64> = input
            .files
            .iter()
            .map(|f| (f.clone(), (1.0 - damping) / n as f64))
            .collect();

        for (node, neighbors) in &input.forward {
            if neighbors.is_empty() {
                continue;
            }
            let share = rank.get(node).copied().unwrap_or(0.0) / neighbors.len() as f64;
            for neighbor in neighbors {
                if let Some(nr) = new_rank.get_mut(neighbor) {
                    *nr += damping * share;
                }
            }
        }

        rank = new_rank;
    }

    rank
}

pub fn top_files(conn: &Connection, limit: usize) -> Vec<(String, f64)> {
    let input = PageRankInput::from_connection(conn);
    let ranks = compute(&input, 0.85, 30);
    let mut sorted: Vec<(String, f64)> = ranks.into_iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    sorted.truncate(limit);
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::property_graph::{CodeGraph, Edge, EdgeKind, Node};

    #[test]
    fn pagerank_basic() {
        let g = CodeGraph::open_in_memory().unwrap();
        let a = g.upsert_node(&Node::file("a.rs")).unwrap();
        let b = g.upsert_node(&Node::file("b.rs")).unwrap();
        let c = g.upsert_node(&Node::file("c.rs")).unwrap();

        g.upsert_edge(&Edge::new(a, b, EdgeKind::Imports)).unwrap();
        g.upsert_edge(&Edge::new(a, c, EdgeKind::Imports)).unwrap();
        g.upsert_edge(&Edge::new(b, c, EdgeKind::Imports)).unwrap();

        let input = PageRankInput::from_connection(g.connection());
        let ranks = compute(&input, 0.85, 30);

        assert_eq!(ranks.len(), 3);
        let rank_c = ranks.get("c.rs").copied().unwrap_or(0.0);
        let rank_a = ranks.get("a.rs").copied().unwrap_or(0.0);
        assert!(
            rank_c > rank_a,
            "c.rs should rank higher (more incoming): c={rank_c} a={rank_a}"
        );
    }

    #[test]
    fn top_files_limit() {
        let g = CodeGraph::open_in_memory().unwrap();
        for i in 0..10 {
            g.upsert_node(&Node::file(&format!("f{i}.rs"))).unwrap();
        }
        let top = top_files(g.connection(), 3);
        assert!(top.len() <= 3);
    }

    #[test]
    fn empty_graph() {
        let g = CodeGraph::open_in_memory().unwrap();
        let top = top_files(g.connection(), 10);
        assert!(top.is_empty());
    }
}
