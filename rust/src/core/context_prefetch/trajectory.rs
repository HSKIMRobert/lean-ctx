//! First-order Markov file-access trajectory for context prefetching.
//!
//! Records recent file transitions and predicts likely next files from
//! empirical transition probabilities.

use std::cmp::Ordering;
use std::collections::HashMap;

/// A trajectory of file accesses.
#[derive(Debug, Clone)]
pub struct FileTrajectory {
    /// Ordered list of recently accessed files.
    pub accesses: Vec<String>,
    /// Transition counts: (from, to) -> count.
    transitions: HashMap<(String, String), u32>,
    /// Maximum sequence length to maintain.
    max_length: usize,
}

impl Default for FileTrajectory {
    fn default() -> Self {
        Self::new(100)
    }
}

impl FileTrajectory {
    /// Create a trajectory buffer capped at `max_length` recent accesses.
    ///
    /// `max_length` must be > 0 for [`Self::record`] to retain history.
    pub fn new(max_length: usize) -> Self {
        Self {
            accesses: Vec::with_capacity(max_length),
            transitions: HashMap::new(),
            max_length,
        }
    }

    /// Record a file access.
    pub fn record(&mut self, path: &str) {
        if self.max_length == 0 {
            return;
        }

        if let Some(previous) = self.accesses.last() {
            if previous != path {
                *self
                    .transitions
                    .entry((previous.clone(), path.to_owned()))
                    .or_insert(0) += 1;
            }
        }
        self.accesses.push(path.to_owned());

        if self.accesses.len() > self.max_length {
            let removed = self.accesses.remove(0);
            if let Some(next) = self.accesses.first() {
                let key = (removed, next.clone());
                if let Some(count) = self.transitions.get_mut(&key) {
                    *count -= 1;
                    if *count == 0 {
                        self.transitions.remove(&key);
                    }
                }
            }
        }
    }

    /// Predict next N most likely files based on transition probabilities.
    /// Uses first-order Markov chain: P(next | current) =
    /// count(current→next) / sum(count(current→*)).
    pub fn predict(&self, n: usize) -> Vec<(String, f64)> {
        let Some(current) = self.accesses.last() else {
            return Vec::new();
        };

        let total: u32 = self
            .transitions
            .iter()
            .filter(|((from, _), _)| from == current)
            .map(|(_, count)| count)
            .sum();
        if total == 0 {
            return Vec::new();
        }

        let mut predictions: Vec<(String, f64)> = self
            .transitions
            .iter()
            .filter(|((from, _), _)| from == current)
            .map(|((_, to), count)| (to.clone(), f64::from(*count) / f64::from(total)))
            .collect();
        predictions.sort_by(|(path_a, probability_a), (path_b, probability_b)| {
            probability_b
                .partial_cmp(probability_a)
                .unwrap_or(Ordering::Equal)
                .then_with(|| path_a.cmp(path_b))
        });
        predictions.truncate(n);
        predictions
    }
}

/// Predict the next most likely files from a trajectory's current state.
///
/// Delegates to [`FileTrajectory::predict`] and returns up to
/// `max_predictions` `(path, probability)` pairs.
pub(crate) fn predict_next_files(
    trajectory: &FileTrajectory,
    max_predictions: usize,
) -> Vec<(String, f64)> {
    trajectory.predict(max_predictions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_trajectory_predicts_nothing() {
        let trajectory = FileTrajectory::new(10);
        assert!(trajectory.predict(3).is_empty());
    }

    #[test]
    fn single_access_predicts_nothing() {
        let mut trajectory = FileTrajectory::new(10);
        trajectory.record("src/a.rs");
        assert!(trajectory.predict(3).is_empty());
    }

    #[test]
    fn repeated_pattern_predicts_correctly() {
        let mut trajectory = FileTrajectory::new(10);
        for path in ["src/a.rs", "src/b.rs", "src/a.rs", "src/b.rs", "src/a.rs"] {
            trajectory.record(path);
        }

        assert_eq!(trajectory.predict(1), vec![("src/b.rs".to_owned(), 1.0)]);
    }

    #[test]
    fn predict_returns_at_most_n() {
        let mut trajectory = FileTrajectory::new(20);
        for path in [
            "src/a.rs", "src/b.rs", "src/a.rs", "src/c.rs", "src/a.rs", "src/d.rs", "src/a.rs",
        ] {
            trajectory.record(path);
        }

        assert_eq!(trajectory.predict(2).len(), 2);
    }

    #[test]
    fn bounded_trajectory_discards_evicted_transitions() {
        let mut trajectory = FileTrajectory::new(3);
        for path in ["src/a.rs", "src/b.rs", "src/c.rs", "src/a.rs"] {
            trajectory.record(path);
        }

        assert_eq!(trajectory.accesses, ["src/b.rs", "src/c.rs", "src/a.rs"]);
        assert!(trajectory.predict(1).is_empty());
    }
}
