//! On-Demand Graph Expansion (F6) — k-hop subgraph extraction.
//!
//! Extracts local neighborhoods from the symbol graph, providing just enough
//! context to understand call chains without loading the entire graph.
#![allow(clippy::duplicated_attributes, unreachable_pub)]

mod hops;
mod partial;

#[allow(unused_imports)]
pub(crate) use hops::{NeighborNode, expand_neighborhood};
pub use partial::PartialGraph;
