//! Embedding engine access + status/reset/reindex handlers.
//! Split out of `ctx_knowledge/mod.rs`; `use super::*` re-imports parent items.

#[allow(clippy::wildcard_imports)]
use super::*;
#[cfg(feature = "embeddings")]
pub(crate) fn embeddings_auto_download_allowed() -> bool {
    std::env::var("LEAN_CTX_EMBEDDINGS_AUTO_DOWNLOAD")
        .ok()
        .is_some_and(|v| {
            matches!(
                v.trim().to_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

#[cfg(feature = "embeddings")]
pub(crate) fn embedding_engine() -> Option<&'static EmbeddingEngine> {
    embedding_engine_impl(false)
}

/// Non-blocking: returns engine only if already loaded. Never triggers model load.
#[cfg(feature = "embeddings")]
pub(crate) fn embedding_engine_nonblocking() -> Option<&'static EmbeddingEngine> {
    embedding_engine_impl(true)
}

#[cfg(feature = "embeddings")]
pub(crate) fn embedding_engine_impl(nonblocking: bool) -> Option<&'static EmbeddingEngine> {
    let cfg = crate::core::config::Config::load();
    let profile = crate::core::config::MemoryProfile::effective(&cfg);
    if !profile.embeddings_enabled() {
        return None;
    }
    if !EmbeddingEngine::is_available() && !embeddings_auto_download_allowed() {
        return None;
    }
    if nonblocking {
        crate::core::embeddings::try_shared_engine()
    } else {
        crate::core::embeddings::shared_engine()
    }
}

pub(crate) fn handle_embeddings_status(project_root: &str) -> String {
    #[cfg(feature = "embeddings")]
    {
        let knowledge = ProjectKnowledge::load_or_create(project_root);
        let model_available = EmbeddingEngine::is_available();
        let auto = embeddings_auto_download_allowed();

        let entries = crate::core::knowledge_embedding::KnowledgeEmbeddingIndex::load(
            &knowledge.project_hash,
        )
        .map_or(0, |i| i.entries.len());

        let path = crate::core::data_dir::lean_ctx_data_dir()
            .ok()
            .map(|d| {
                d.join("knowledge")
                    .join(&knowledge.project_hash)
                    .join("embeddings.json")
            })
            .map_or_else(|| "<unknown>".to_string(), |p| p.display().to_string());

        format!(
            "Knowledge embeddings: model={}, auto_download={}, index_entries={}, path={path}",
            if model_available {
                "present"
            } else {
                "missing"
            },
            if auto { "on" } else { "off" },
            entries
        )
    }
    #[cfg(not(feature = "embeddings"))]
    {
        let _ = project_root;
        "ERR: embeddings feature not enabled".to_string()
    }
}

pub(crate) fn handle_embeddings_reset(project_root: &str) -> String {
    #[cfg(feature = "embeddings")]
    {
        let knowledge = ProjectKnowledge::load_or_create(project_root);
        match crate::core::knowledge_embedding::reset(&knowledge.project_hash) {
            Ok(()) => "Embeddings index reset.".to_string(),
            Err(e) => format!("Embeddings reset failed: {e}"),
        }
    }
    #[cfg(not(feature = "embeddings"))]
    {
        let _ = project_root;
        "ERR: embeddings feature not enabled".to_string()
    }
}

pub(crate) fn handle_embeddings_reindex(project_root: &str) -> String {
    #[cfg(feature = "embeddings")]
    {
        let Some(knowledge) = ProjectKnowledge::load(project_root) else {
            return "No knowledge stored for this project yet.".to_string();
        };
        let policy = match load_policy_or_error() {
            Ok(p) => p,
            Err(e) => return e,
        };

        let Some(engine) = embedding_engine() else {
            return "Embeddings model not available. Set LEAN_CTX_EMBEDDINGS_AUTO_DOWNLOAD=1 to allow auto-download, then re-run."
                    .to_string();
        };

        let mut idx =
            crate::core::knowledge_embedding::KnowledgeEmbeddingIndex::new(&knowledge.project_hash);

        let mut facts: Vec<&crate::core::knowledge::KnowledgeFact> =
            knowledge.facts.iter().filter(|f| f.is_current()).collect();
        facts.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.last_confirmed.cmp(&a.last_confirmed))
                .then_with(|| a.category.cmp(&b.category))
                .then_with(|| a.key.cmp(&b.key))
        });

        let max = policy.embeddings.max_facts;
        let mut embedded = 0usize;
        for f in facts.into_iter().take(max) {
            if crate::core::knowledge_embedding::embed_and_store(
                &mut idx,
                engine,
                &f.category,
                &f.key,
                &f.value,
            )
            .is_ok()
            {
                embedded += 1;
            }
        }

        crate::core::knowledge_embedding::compact_against_knowledge(&mut idx, &knowledge, &policy);
        match idx.save() {
            Ok(()) => format!("Embeddings reindex ok (embedded {embedded} facts)."),
            Err(e) => format!("Embeddings reindex failed: {e}"),
        }
    }
    #[cfg(not(feature = "embeddings"))]
    {
        let _ = project_root;
        "ERR: embeddings feature not enabled".to_string()
    }
}
