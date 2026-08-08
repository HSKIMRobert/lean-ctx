//! On-Demand Graph Expansion (F6) — k-hop subgraph extraction.
//!
//! Extracts local neighborhoods from the symbol graph, providing just enough
//! context to understand call chains without loading the entire graph.

mod hops;
mod partial;

#[cfg(test)]
pub(crate) use hops::expand_neighborhood;
#[cfg(test)]
pub(crate) use partial::EdgeKind;
pub use partial::PartialGraph;
