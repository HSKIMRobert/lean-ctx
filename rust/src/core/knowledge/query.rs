use chrono::{DateTime, Utc};

use super::ranking::sort_fact_for_output;
use super::types::{KnowledgeFact, ProjectKnowledge};

impl ProjectKnowledge {
    fn matching_indices(&self, term: &str, include_session: bool) -> Vec<usize> {
        let Some(indices) = self.index.token_positions.get(term) else {
            return if include_session {
                self.index
                    .session_token_positions
                    .get(term)
                    .cloned()
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
        };

        if !include_session {
            return indices.clone();
        }

        let Some(session_indices) = self.index.session_token_positions.get(term) else {
            return indices.clone();
        };
        let mut merged = indices.clone();
        merged.extend(
            session_indices
                .iter()
                .copied()
                .filter(|idx| indices.binary_search(idx).is_err()),
        );
        merged
    }

    pub fn recall(&self, query: &str) -> Vec<&KnowledgeFact> {
        let q = query.to_lowercase();
        let terms: Vec<&str> = q.split_whitespace().collect();
        if terms.is_empty() {
            return Vec::new();
        }

        let mut match_counts: std::collections::HashMap<usize, usize> =
            std::collections::HashMap::new();
        for term in &terms {
            for idx in self.matching_indices(term, true) {
                if self.facts[idx].is_current() {
                    *match_counts.entry(idx).or_insert(0) += 1;
                }
            }
        }

        let mut results: Vec<(&KnowledgeFact, f32)> = match_counts
            .into_iter()
            .map(|(idx, count)| {
                let f = &self.facts[idx];
                let relevance = (count as f32 / terms.len() as f32) * f.quality_score();
                (f, relevance)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().map(|(f, _)| f).collect()
    }

    pub fn recall_by_category(&self, category: &str) -> Vec<&KnowledgeFact> {
        self.index
            .category_positions
            .get(category)
            .into_iter()
            .flatten()
            .filter_map(|&idx| self.facts.get(idx))
            .filter(|f| f.is_current())
            .collect()
    }

    pub fn recall_at_time(&self, query: &str, at: DateTime<Utc>) -> Vec<&KnowledgeFact> {
        let q = query.to_lowercase();
        let terms: Vec<&str> = q.split_whitespace().collect();
        if terms.is_empty() {
            return Vec::new();
        }

        let mut match_counts: std::collections::HashMap<usize, usize> =
            std::collections::HashMap::new();
        for term in &terms {
            for idx in self.matching_indices(term, false) {
                if self.facts[idx].was_valid_at(at) {
                    *match_counts.entry(idx).or_insert(0) += 1;
                }
            }
        }

        let mut results: Vec<(&KnowledgeFact, f32)> = match_counts
            .into_iter()
            .map(|(idx, count)| {
                let f = &self.facts[idx];
                (f, count as f32 / terms.len() as f32)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().map(|(f, _)| f).collect()
    }

    pub fn timeline(&self, category: &str) -> Vec<&KnowledgeFact> {
        let mut facts: Vec<&KnowledgeFact> = self
            .index
            .category_positions
            .get(category)
            .into_iter()
            .flatten()
            .filter_map(|&idx| self.facts.get(idx))
            .collect();
        facts.sort_by_key(|x| x.created_at);
        facts
    }

    pub fn list_rooms(&self) -> Vec<(String, usize)> {
        let mut categories: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for f in &self.facts {
            if f.is_current() {
                *categories.entry(f.category.clone()).or_insert(0) += 1;
            }
        }
        categories.into_iter().collect()
    }

    pub fn recall_for_output(&mut self, query: &str, limit: usize) -> (Vec<KnowledgeFact>, usize) {
        let q = query.to_lowercase();
        let terms: Vec<&str> = q.split_whitespace().filter(|t| !t.is_empty()).collect();
        if terms.is_empty() {
            return (Vec::new(), 0);
        }

        let mut match_counts: std::collections::HashMap<usize, usize> =
            std::collections::HashMap::new();
        for term in &terms {
            for idx in self.matching_indices(term, true) {
                if self.facts[idx].is_current() {
                    *match_counts.entry(idx).or_insert(0) += 1;
                }
            }
        }

        struct Scored {
            idx: usize,
            relevance: f32,
        }

        let mut scored: Vec<Scored> = match_counts
            .into_iter()
            .map(|(idx, count)| {
                let f = &self.facts[idx];
                let mut relevance = (count as f32 / terms.len() as f32) * f.confidence;
                // Exact-match boost: an exact hit on the fact key (or category)
                // should rank above incidental lexical matches (#2363). The +1.0
                // dominates the [0,1] coverage*confidence base.
                let key_lower = f.key.to_lowercase();
                if key_lower == q {
                    relevance += 1.0;
                } else if f.category.to_lowercase() == q {
                    relevance += 0.5;
                }
                // Observation tier (#802): a relevant synthesized entity-summary is
                // orientation — lift it above incidental matches, but keep it below an
                // exact key hit (+1.0) so a stale summary never buries a precise raw
                // fact. Balanced, not absolute.
                if f.is_synthesized_observation() {
                    relevance += 0.4;
                }
                Scored { idx, relevance }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| sort_fact_for_output(&self.facts[a.idx], &self.facts[b.idx]))
        });

        let total = scored.len();
        scored.truncate(limit);

        let now = Utc::now();
        let mut out: Vec<KnowledgeFact> = Vec::new();
        for s in scored {
            if let Some(f) = self.facts.get_mut(s.idx) {
                f.retrieval_count = f.retrieval_count.saturating_add(1);
                f.last_retrieved = Some(now);
                out.push(f.clone());
            }
        }

        (out, total)
    }

    pub fn recall_by_category_for_output(
        &mut self,
        category: &str,
        limit: usize,
    ) -> (Vec<KnowledgeFact>, usize) {
        let mut idxs: Vec<usize> = self
            .index
            .category_positions
            .get(category)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&idx| self.facts[idx].is_current())
            .collect();

        // Within a category, synthesized observation summaries lead (#802) — a
        // balanced tier ahead of the usual salience sort, never an absolute override.
        idxs.sort_by(|a, b| {
            let (fa, fb) = (&self.facts[*a], &self.facts[*b]);
            fb.is_synthesized_observation()
                .cmp(&fa.is_synthesized_observation())
                .then_with(|| sort_fact_for_output(fa, fb))
        });

        let total = idxs.len();
        idxs.truncate(limit);

        let now = Utc::now();
        let mut out = Vec::new();
        for idx in idxs {
            if let Some(f) = self.facts.get_mut(idx) {
                f.retrieval_count = f.retrieval_count.saturating_add(1);
                f.last_retrieved = Some(now);
                out.push(f.clone());
            }
        }

        (out, total)
    }
}
