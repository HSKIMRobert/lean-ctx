//! MDL Read Mode (F8) — structural descriptions for compact code representation.
//!
//! Generates minimum-length structural fingerprints of source files using the
//! Minimum Description Length principle.

mod bounds;
mod structural;

pub use structural::StructuralDescription;
#[cfg(test)]
pub(crate) use structural::generate_structural_description;
