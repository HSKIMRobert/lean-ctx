//! Wasserstein Token Allocator (F4) — Optimal Transport for context budgets.
//!
//! Distributes a global token budget across files using entropy-regularized OT.

mod allocator;
mod transport;

pub use allocator::TokenAllocation;
#[allow(unused_imports)]
pub(crate) use allocator::allocate_budget;
#[allow(unused_imports)]
pub(crate) use transport::{CostEntry, sinkhorn_plan};
