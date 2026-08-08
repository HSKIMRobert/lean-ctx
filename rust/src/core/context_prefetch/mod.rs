//! Predictive Context Prefetch (F2) — trajectory-based preloading.
//!
//! Unifies predictive prefetch, FEP prefetch, and active inference into a
//! coherent prefetch pipeline with trajectory prediction.

mod preloader;
mod trajectory;

pub use preloader::PrefetchPlan;
#[allow(unused_imports)]
pub(crate) use preloader::build_prefetch_plan;
#[allow(unused_imports)]
pub(crate) use trajectory::{FileTrajectory, predict_next_files};
