//! MDL Read Mode (F8) — structural descriptions for compact code representation.
//!
//! Generates minimum-length structural fingerprints of source files using the
//! Minimum Description Length principle.
#![allow(unreachable_pub)]

mod bounds;
mod structural;

#[allow(unused_imports)]
pub(crate) use bounds::{compression_ratio, description_length};
pub use structural::StructuralDescription;
#[allow(unused_imports)]
pub(crate) use structural::generate_structural_description;
