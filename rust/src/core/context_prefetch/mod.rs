//! Predictive Context Prefetch (F2) — trajectory-based preloading.
//!
//! Unifies predictive prefetch, FEP prefetch, and active inference into a
//! coherent prefetch pipeline with trajectory prediction.

mod preloader;
mod trajectory;

pub use preloader::PrefetchPlan;
#[cfg(test)]
pub(crate) use preloader::build_prefetch_plan;
#[cfg(test)]
pub(crate) use trajectory::FileTrajectory;
