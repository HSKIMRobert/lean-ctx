//! FSRS-5 spaced-repetition model for knowledge-fact retention scheduling.
//!
//! Implements retrievability decay, stability updates, and optimal review
//! intervals from the open-source FSRS-5 weight set.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// FSRS-5 model weights from the open-source FSRS research.
const W: [f64; 17] = [
    0.4, 0.6, 2.4, 5.8, // Initial stability (Again, Hard, Good, Easy).
    4.93, 0.94, 0.86, 0.01, // Difficulty.
    1.49, 0.14, 0.94, 2.18, // Stability update.
    0.05, 0.34, 1.26, 0.29, // Additional.
    2.61, // Stability decay.
];

/// Retention state for a knowledge fact under the FSRS-5 model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryState {
    /// Unique identifier for the knowledge fact.
    pub fact_key: String,
    /// Number of days at which retrievability drops to 90 percent.
    pub stability: f64,
    /// Intrinsic difficulty of this fact, normalized to `0.0..=1.0`.
    pub difficulty: f64,
    /// Timestamp of the last review or use.
    pub last_review: DateTime<Utc>,
    /// Total number of reviews.
    pub review_count: u32,
    /// Review ratings where 1=Again, 2=Hard, 3=Good, and 4=Easy.
    #[serde(default)]
    pub rating_history: Vec<u8>,
}

/// Compute the probability that a fact is still remembered at `now`.
///
/// Based on FSRS-5: `R(t) = (1 + t / (9 · S))^(-1)` where `t` is elapsed
/// days since [`MemoryState::last_review`] and `S` is [`MemoryState::stability`].
pub(crate) fn retrievability(state: &MemoryState, now: DateTime<Utc>) -> f64 {
    let elapsed_days = (now - state.last_review).num_seconds() as f64 / 86_400.0;
    if elapsed_days <= 0.0 || state.stability <= 0.0 {
        return 1.0;
    }
    (1.0 + elapsed_days / (9.0 * state.stability)).powf(-1.0)
}

/// Update a memory state after a review using a rating in the range 1 through 4.
pub(crate) fn update_stability(state: &mut MemoryState, rating: u8) {
    update_stability_at(state, rating, Utc::now());
}

fn update_stability_at(state: &mut MemoryState, rating: u8, now: DateTime<Utc>) {
    let rating = rating.clamp(1, 4);
    let current_retrievability = retrievability(state, now);
    let old_stability = state.stability.max(f64::EPSILON);
    let old_difficulty = state.difficulty;

    state.difficulty =
        (W[4] - W[5] * (f64::from(rating) - 3.0) + W[6] * (old_difficulty - W[4])).clamp(0.0, 1.0);

    state.stability = if rating == 1 {
        let forgotten_stability = W[9]
            * state.difficulty.max(f64::EPSILON).powf(-W[10])
            * ((old_stability + 1.0).powf(W[11]) - 1.0)
            * W[12].exp();
        forgotten_stability
            .max(f64::EPSILON)
            .min(old_stability * 0.9)
    } else {
        let stability_gain = W[8].exp()
            * (11.0 - state.difficulty)
            * old_stability.powf(-W[13])
            * ((W[14] * (1.0 - current_retrievability)).exp() - 1.0);
        old_stability * (1.0 + stability_gain.max(0.0))
    };

    state.last_review = now;
    state.review_count = state.review_count.saturating_add(1);
    state.rating_history.push(rating);
    if state.rating_history.len() > 50 {
        let drain_count = state.rating_history.len() - 50;
        state.rating_history.drain(0..drain_count);
    }
}

/// Compute the interval in days that reaches `target_retention`.
///
/// Inverts the FSRS-5 retrievability curve:
/// `t = 9 · S · (1 / R_target - 1)` for `R_target` in `(0, 1)`.
pub(crate) fn optimal_interval(state: &MemoryState, target_retention: f64) -> f64 {
    if target_retention <= 0.0 || target_retention >= 1.0 || state.stability <= 0.0 {
        return 0.0;
    }
    9.0 * state.stability * (1.0 / target_retention - 1.0)
}

/// Create an initial memory state from a fact key and first-review rating.
pub(crate) fn initial_state(fact_key: String, rating: u8) -> MemoryState {
    let rating = rating.clamp(1, 4);
    let rating_index = usize::from(rating - 1);
    MemoryState {
        fact_key,
        stability: W[rating_index],
        difficulty: (W[4] - W[5] * (f64::from(rating) - 3.0)).clamp(0.0, 1.0),
        last_review: Utc::now(),
        review_count: 1,
        rating_history: vec![rating],
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::core::memory_scheduler::fsrs::{
        initial_state, optimal_interval, retrievability, update_stability_at,
    };

    #[test]
    fn retrievability_is_one_immediately_after_review() {
        let now = Utc::now();
        let mut state = initial_state("fact".to_string(), 3);
        state.last_review = now;

        assert!((retrievability(&state, now) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn retrievability_decreases_over_time() {
        let now = Utc::now();
        let mut state = initial_state("fact".to_string(), 3);
        state.last_review = now;

        let after_one_day = retrievability(&state, now + Duration::days(1));
        let after_ten_days = retrievability(&state, now + Duration::days(10));
        assert!(after_ten_days < after_one_day);
    }

    #[test]
    fn retrievability_higher_stability_decays_slower() {
        let now = Utc::now();
        let mut low = initial_state("low".to_string(), 1);
        low.last_review = now;
        let mut high = initial_state("high".to_string(), 4);
        high.last_review = now;

        let review_time = now + Duration::days(7);
        assert!(retrievability(&high, review_time) > retrievability(&low, review_time));
    }

    #[test]
    fn optimal_interval_for_90_percent_retention() {
        let state = initial_state("fact".to_string(), 3);

        assert!((optimal_interval(&state, 0.9) - state.stability).abs() < 1.0e-12);
    }

    #[test]
    fn update_stability_increases_on_good_rating() {
        let now = Utc::now();
        let mut state = initial_state("fact".to_string(), 3);
        state.last_review = now - Duration::days(10);
        let old_stability = state.stability;

        update_stability_at(&mut state, 3, now);

        assert!(state.stability > old_stability);
    }

    #[test]
    fn update_stability_decreases_on_again_rating() {
        let now = Utc::now();
        let mut state = initial_state("fact".to_string(), 4);
        state.last_review = now - Duration::days(10);
        let old_stability = state.stability;

        update_stability_at(&mut state, 1, now);

        assert!(state.stability < old_stability);
    }

    #[test]
    fn initial_state_sets_correct_stability_for_each_rating() {
        let expected = [0.4, 0.6, 2.4, 5.8];

        for (rating, stability) in (1_u8..=4).zip(expected) {
            let state = initial_state(format!("fact-{rating}"), rating);
            assert!((state.stability - stability).abs() < f64::EPSILON);
        }
    }
}
