//! Wasserstein/Sinkhorn token budget allocation across context files.
//!
//! Distributes a fixed token budget proportionally to relevance scores using
//! optimal transport from a single supply node to per-file demand nodes.

use super::transport::sinkhorn_plan;

/// Allocation result for a single file or context chunk.
#[derive(Debug, Clone)]
pub struct TokenAllocation {
    /// File path or chunk identifier.
    pub target: String,
    /// Allocated token budget.
    pub tokens: usize,
    /// Fraction of the total budget, in the range `0.0..=1.0`.
    pub fraction: f64,
}

/// Allocates a token budget across files according to their relevance scores.
///
/// Each input tuple is `(path, current_tokens, relevance_score)`. Relevance is
/// clamped to `0.0..=1.0`; when every score is zero, the budget is shared
/// uniformly. `current_tokens` is retained for callers that use it to describe
/// source size, but it does not cap the requested context budget.
pub(crate) fn allocate_budget(
    files: &[(&str, usize, f64)],
    total_budget: usize,
) -> Vec<TokenAllocation> {
    if files.is_empty() {
        return Vec::new();
    }

    let relevance: Vec<f64> = files
        .iter()
        .map(|(_, _, score)| sanitize_relevance(*score))
        .collect();
    let relevance_sum: f64 = relevance.iter().sum();
    let weights = if relevance_sum > 0.0 {
        relevance.clone()
    } else {
        vec![1.0; files.len()]
    };
    let weight_sum: f64 = weights.iter().sum();
    let demand: Vec<f64> = weights
        .iter()
        .map(|weight| total_budget as f64 * weight / weight_sum)
        .collect();
    let cost_matrix = vec![relevance.iter().map(|score| 1.0 - score).collect()];
    let plan = sinkhorn_plan(&[total_budget as f64], &demand, &cost_matrix, 0.1, 50);

    let mut transported = vec![0.0; files.len()];
    for (_, target_idx, amount) in &plan {
        if let Some(target) = transported.get_mut(*target_idx) {
            *target += amount;
        }
    }
    if plan.is_empty() || transported.iter().all(|amount| *amount == 0.0) {
        transported = demand;
    }
    let integer_allocations = round_allocations(&transported, total_budget);

    files
        .iter()
        .zip(integer_allocations)
        .map(|((target, _, _), tokens)| TokenAllocation {
            target: (*target).to_owned(),
            tokens,
            fraction: if total_budget == 0 {
                0.0
            } else {
                tokens as f64 / total_budget as f64
            },
        })
        .collect()
}

fn sanitize_relevance(score: f64) -> f64 {
    if score.is_finite() {
        score.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn round_allocations(values: &[f64], budget: usize) -> Vec<usize> {
    let mut rounded: Vec<usize> = values
        .iter()
        .map(|value| value.max(0.0).floor() as usize)
        .collect();
    let assigned = rounded.iter().sum::<usize>().min(budget);
    let remaining = budget - assigned;
    let mut by_remainder: Vec<usize> = (0..values.len()).collect();
    by_remainder.sort_by(|left, right| {
        let left_remainder = values[*left] - values[*left].floor();
        let right_remainder = values[*right] - values[*right].floor();
        right_remainder
            .total_cmp(&left_remainder)
            .then_with(|| left.cmp(right))
    });

    for index in by_remainder.into_iter().take(remaining) {
        rounded[index] = rounded[index].saturating_add(1);
    }
    rounded
}

#[cfg(test)]
mod tests {
    use super::allocate_budget;

    #[test]
    fn single_file_gets_full_budget() {
        let allocations = allocate_budget(&[("src/lib.rs", 20, 0.2)], 100);
        assert_eq!(allocations[0].tokens, 100);
        assert_eq!(allocations[0].fraction, 1.0);
    }

    #[test]
    fn irrelevant_file_gets_minimum() {
        let allocations = allocate_budget(
            &[("relevant.rs", 100, 1.0), ("irrelevant.rs", 100, 0.0)],
            100,
        );
        assert_eq!(allocations[1].tokens, 0);
    }

    #[test]
    fn allocation_sums_to_budget() {
        let allocations = allocate_budget(
            &[("a.rs", 100, 0.7), ("b.rs", 80, 0.2), ("c.rs", 40, 0.1)],
            101,
        );
        assert_eq!(
            allocations.iter().map(|entry| entry.tokens).sum::<usize>(),
            101
        );
    }

    #[test]
    fn higher_relevance_gets_more_tokens() {
        let allocations = allocate_budget(&[("high.rs", 100, 0.9), ("low.rs", 100, 0.1)], 100);
        assert!(allocations[0].tokens > allocations[1].tokens);
    }
}
