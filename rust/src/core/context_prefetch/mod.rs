//! Predictive Context Prefetch (F2) — trajectory-based preloading.
//!
//! Unifies predictive prefetch, FEP prefetch, and active inference into a
//! coherent prefetch pipeline with trajectory prediction.

mod preloader;
mod trajectory;
mod warming;

pub use preloader::PrefetchPlan;
#[cfg(test)]
pub(crate) use preloader::build_prefetch_plan;
pub(crate) use trajectory::FileTrajectory;
pub(crate) use warming::{skipped_count, warm_predictions, warmed_count};
