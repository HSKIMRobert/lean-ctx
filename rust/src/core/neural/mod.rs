//! Neural context compression — trained models replacing heuristic filters.
//!
//! Feature-gated under `#[cfg(feature = "neural")]`.
//! When an ONNX model is present, switches from heuristic to neural scoring.
//! Falls back gracefully to heuristic mode when no model is available.

#[allow(dead_code)]
pub(crate) mod attention_learned;
#[allow(dead_code)]
pub(crate) mod cache_alignment;
#[allow(dead_code)]
pub(crate) mod context_reorder;
#[allow(dead_code)]
pub(crate) mod line_scorer;
#[allow(dead_code)]
pub(crate) mod token_optimizer;

use std::path::PathBuf;

use attention_learned::LearnedAttention;
use line_scorer::NeuralLineScorer;
use token_optimizer::TokenOptimizer;

pub(crate) struct NeuralEngine {
    line_scorer: Option<NeuralLineScorer>,
    token_optimizer: TokenOptimizer,
    attention: LearnedAttention,
}

impl NeuralEngine {
    pub(crate) fn load() -> Self {
        let model_dir = Self::model_directory();

        let line_scorer = if model_dir.join("line_importance.onnx").exists() {
            match NeuralLineScorer::load(&model_dir.join("line_importance.onnx")) {
                Ok(scorer) => {
                    tracing::info!("Neural line scorer loaded from {:?}", model_dir);
                    Some(scorer)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load neural line scorer: {e}. Using heuristic fallback."
                    );
                    None
                }
            }
        } else {
            tracing::debug!("No ONNX model found, using heuristic line scoring");
            None
        };

        let token_optimizer = TokenOptimizer::load_or_default(&model_dir);
        let attention = LearnedAttention::load_or_default(&model_dir);

        Self {
            line_scorer,
            token_optimizer,
            attention,
        }
    }

    pub(crate) fn score_line(&self, line: &str, position: f64, task_keywords: &[String]) -> f64 {
        if let Some(ref scorer) = self.line_scorer {
            scorer.score_line(line, position, task_keywords)
        } else {
            self.heuristic_score(line, position)
        }
    }

    pub(crate) fn optimize_line(&self, line: &str) -> String {
        self.token_optimizer.optimize_line(line)
    }

    pub(crate) fn attention_weight(&self, position: f64) -> f64 {
        self.attention.weight(position)
    }

    pub(crate) fn has_neural_model(&self) -> bool {
        self.line_scorer.is_some()
    }

    fn heuristic_score(&self, line: &str, position: f64) -> f64 {
        let structural = super::attention_model::structural_importance(line);
        let positional = self.attention.weight(position);
        (structural * positional).sqrt()
    }

    fn model_directory() -> PathBuf {
        if let Ok(dir) = std::env::var("LEAN_CTX_MODELS_DIR") {
            return PathBuf::from(dir);
        }

        if let Ok(d) = crate::core::paths::cache_dir() {
            return d.join("models");
        }

        PathBuf::from("models")
    }
}
