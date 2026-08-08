//! Context prefetch planning from file-access trajectory predictions.
//!
//! Filters low-confidence and already-loaded files to build a bounded
//! preload plan for proactive context warming.

use super::trajectory::FileTrajectory;

/// A prefetch plan: files to preload and their predicted relevance.
#[derive(Debug, Clone)]
pub struct PrefetchPlan {
    /// Files selected for prefetch, highest confidence first.
    pub files: Vec<PrefetchEntry>,
    /// Sum of estimated token sizes; 0 until size integration is wired.
    pub total_predicted_tokens: usize,
}

/// One file candidate in a prefetch plan.
#[derive(Debug, Clone)]
pub struct PrefetchEntry {
    /// File path to preload.
    pub path: String,
    /// Transition probability in `(0.0, 1.0]`.
    pub confidence: f64,
    /// Human-readable selection rationale.
    pub reason: &'static str,
}

/// Build a prefetch plan from trajectory predictions and co-access data.
///
/// Predictions at or below `min_confidence` and files already present in the
/// current context are excluded.
pub(crate) fn build_prefetch_plan(
    trajectory: &FileTrajectory,
    loaded_files: &[&str],
    max_files: usize,
    min_confidence: f64,
) -> PrefetchPlan {
    let files: Vec<PrefetchEntry> = trajectory
        .predict(max_files.saturating_add(loaded_files.len()))
        .into_iter()
        .filter(|(path, confidence)| {
            *confidence > min_confidence && !loaded_files.contains(&path.as_str())
        })
        .take(max_files)
        .map(|(path, confidence)| PrefetchEntry {
            path,
            confidence,
            reason: "trajectory transition",
        })
        .collect();
    // Estimate unavailable without reading files; set to 0 until integration wiring provides cached sizes.
    let total_predicted_tokens = 0;

    PrefetchPlan {
        files,
        total_predicted_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_trajectory_gives_empty_plan() {
        let plan = build_prefetch_plan(&FileTrajectory::new(10), &[], 3, 0.2);
        assert!(plan.files.is_empty());
        assert_eq!(plan.total_predicted_tokens, 0);
    }

    #[test]
    fn loaded_files_are_excluded() {
        let mut trajectory = FileTrajectory::new(10);
        for path in ["src/a.rs", "src/b.rs", "src/a.rs"] {
            trajectory.record(path);
        }

        let plan = build_prefetch_plan(&trajectory, &["src/b.rs"], 3, 0.2);
        assert!(plan.files.is_empty());
    }

    #[test]
    fn low_confidence_filtered() {
        let mut trajectory = FileTrajectory::new(10);
        for path in ["src/a.rs", "src/b.rs", "src/a.rs", "src/c.rs", "src/a.rs"] {
            trajectory.record(path);
        }

        let plan = build_prefetch_plan(&trajectory, &[], 3, 0.5);
        assert!(plan.files.is_empty());
    }

    #[test]
    fn selected_files_have_zero_token_estimate_until_wired() {
        let mut trajectory = FileTrajectory::new(10);
        for path in ["src/a.rs", "src/b.rs", "src/a.rs"] {
            trajectory.record(path);
        }

        let plan = build_prefetch_plan(&trajectory, &[], 1, 0.2);
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.total_predicted_tokens, 0);
    }
}
