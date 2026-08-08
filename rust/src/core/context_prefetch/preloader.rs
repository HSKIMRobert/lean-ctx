use super::trajectory::FileTrajectory;

/// A prefetch plan: files to preload and their predicted relevance.
#[derive(Debug, Clone)]
pub struct PrefetchPlan {
    pub files: Vec<PrefetchEntry>,
    pub total_predicted_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct PrefetchEntry {
    pub path: String,
    pub confidence: f64,
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
    let total_predicted_tokens = files
        .iter()
        .map(|entry| crate::core::tokens::count_tokens(&entry.path))
        .sum();

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
    fn selected_files_have_token_estimate() {
        let mut trajectory = FileTrajectory::new(10);
        for path in ["src/a.rs", "src/b.rs", "src/a.rs"] {
            trajectory.record(path);
        }

        let plan = build_prefetch_plan(&trajectory, &[], 1, 0.2);
        assert_eq!(plan.files.len(), 1);
        assert_eq!(
            plan.total_predicted_tokens,
            crate::core::tokens::count_tokens("src/b.rs")
        );
    }
}
