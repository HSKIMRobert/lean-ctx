//! BM25 relevance scoring for information-bottleneck chunk selection.
//!
//! Ranks text chunks against intent-specific query terms as a lightweight
//! proxy for mutual information between task intent and source content.

use std::cmp::Ordering;
use std::collections::HashSet;

use crate::core::tokens::count_tokens;

use super::intent::TaskIntent;

/// Relevance assigned to one input chunk.
#[derive(Debug, Clone)]
pub struct RelevanceScore {
    /// Zero-based position of the chunk in the input slice.
    pub chunk_index: usize,
    /// BM25-derived score normalized to the inclusive range `[0, 1]`.
    pub score: f64,
}

/// Generate query terms for IB compression based on task intent.
/// Each intent has domain-specific terms that maximize mutual information.
pub(crate) fn intent_query_terms(intent: &TaskIntent) -> Vec<&'static str> {
    match intent {
        TaskIntent::Debug => vec![
            "error", "panic", "unwrap", "stack", "trace", "fail", "crash", "assert", "expect",
            "bug",
        ],
        TaskIntent::Refactor => vec![
            "struct",
            "trait",
            "impl",
            "pub",
            "mod",
            "type",
            "fn",
            "signature",
            "interface",
        ],
        TaskIntent::Implement => vec![
            "test",
            "spec",
            "todo",
            "fixme",
            "requirement",
            "feature",
            "api",
            "endpoint",
        ],
        TaskIntent::Review => vec![
            "unsafe",
            "unwrap",
            "todo",
            "hack",
            "fixme",
            "security",
            "performance",
            "complexity",
        ],
        TaskIntent::Explore => vec![
            "mod", "struct", "enum", "trait", "fn", "import", "use", "pub",
        ],
        TaskIntent::Unknown => vec![],
    }
}

/// Score chunks by relevance to the current task intent.
/// Uses BM25-style IDF-weighted term overlap as a lightweight mutual information proxy.
pub(crate) fn compute_relevance(
    chunks: &[&str],
    intent: &TaskIntent,
    explicit_query: Option<&str>,
) -> Vec<RelevanceScore> {
    if chunks.is_empty() {
        return Vec::new();
    }

    let mut terms: Vec<String> = intent_query_terms(intent)
        .into_iter()
        .map(str::to_owned)
        .collect();
    if let Some(query) = explicit_query {
        terms.extend(words(query));
    }
    deduplicate(&mut terms);

    if terms.is_empty() {
        return chunks
            .iter()
            .enumerate()
            .map(|(chunk_index, _)| RelevanceScore {
                chunk_index,
                score: 0.0,
            })
            .collect();
    }

    let chunk_words: Vec<Vec<String>> = chunks.iter().map(|chunk| words(chunk)).collect();
    let lengths: Vec<f64> = chunks
        .iter()
        .map(|chunk| count_tokens(chunk).max(1) as f64)
        .collect();
    let average_length = lengths.iter().sum::<f64>() / lengths.len() as f64;
    let document_count = chunks.len() as f64;
    let document_frequencies: Vec<usize> = terms
        .iter()
        .map(|term| {
            chunk_words
                .iter()
                .filter(|chunk| chunk.iter().any(|word| word == term))
                .count()
        })
        .collect();

    let mut scores: Vec<RelevanceScore> = chunk_words
        .iter()
        .enumerate()
        .map(|(chunk_index, chunk)| {
            let score = bm25_score(
                chunk,
                lengths[chunk_index],
                average_length,
                document_count,
                &terms,
                &document_frequencies,
            );
            RelevanceScore { chunk_index, score }
        })
        .collect();

    normalize(&mut scores);
    scores.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.chunk_index.cmp(&right.chunk_index))
    });
    scores
}

fn words(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn deduplicate(terms: &mut Vec<String>) {
    let mut seen = HashSet::new();
    terms.retain(|term| !term.is_empty() && seen.insert(term.clone()));
}

fn bm25_score(
    chunk: &[String],
    chunk_length: f64,
    average_length: f64,
    document_count: f64,
    terms: &[String],
    document_frequencies: &[usize],
) -> f64 {
    const K1: f64 = 1.2;
    const B: f64 = 0.75;

    terms
        .iter()
        .zip(document_frequencies)
        .filter(|(_, frequency)| **frequency > 0)
        .map(|(term, document_frequency)| {
            let term_frequency = chunk.iter().filter(|word| *word == term).count() as f64;
            let inverse_document_frequency = (document_count / *document_frequency as f64).ln();
            let length_factor = 1.0 - B + B * chunk_length / average_length;
            inverse_document_frequency * term_frequency * (K1 + 1.0)
                / (term_frequency + K1 * length_factor)
        })
        .sum()
}

fn normalize(scores: &mut [RelevanceScore]) {
    let maximum = scores.iter().map(|score| score.score).fold(0.0, f64::max);
    if maximum > 0.0 {
        for score in scores {
            score.score /= maximum;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TaskIntent, compute_relevance};

    #[test]
    fn debug_intent_scores_error_lines_higher() {
        let chunks = ["normal control flow", "panic error crash in parser"];
        let scores = compute_relevance(&chunks, &TaskIntent::Debug, None);
        assert_eq!(scores[0].chunk_index, 1);
        assert!(scores[0].score > scores[1].score);
    }

    #[test]
    fn empty_intent_returns_uniform_scores() {
        let chunks = ["first chunk", "a very different second chunk"];
        let scores = compute_relevance(&chunks, &TaskIntent::Unknown, None);
        assert!(scores.iter().all(|score| score.score == scores[0].score));
    }

    #[test]
    fn explicit_query_overrides_intent() {
        let chunks = ["panic", "database database timeout"];
        let scores = compute_relevance(
            &chunks,
            &TaskIntent::Debug,
            Some("database timeout database"),
        );
        assert_eq!(scores[0].chunk_index, 1);
    }

    #[test]
    fn scores_are_normalized() {
        let chunks = ["panic crash error", "panic", "ordinary text"];
        let scores = compute_relevance(&chunks, &TaskIntent::Debug, None);
        assert!(
            scores
                .iter()
                .all(|score| (0.0..=1.0).contains(&score.score))
        );
        assert!((scores[0].score - 1.0).abs() < f64::EPSILON);
    }
}
