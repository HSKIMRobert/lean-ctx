//! Wasserstein Token Allocator (F4) — Optimal Transport for context budgets.
//!
//! Distributes a global token budget across files using entropy-regularized OT.

mod allocator;
mod transport;

pub use allocator::TokenAllocation;
#[cfg(test)]
pub(crate) use allocator::allocate_budget;
