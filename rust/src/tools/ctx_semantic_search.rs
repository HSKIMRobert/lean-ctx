use std::path::Path;

use crate::core::hybrid_search::{format_hybrid_results, HybridConfig, HybridResult};
use crate::core::telemetry::global_metrics;
use crate::core::vector_index::BM25Index;
use crate::tools::CrpMode;

#[cfg(feature = "embeddings")]
use crate::core::embedding_index::EmbeddingIndex;

pub fn handle(query: &str, path: &str, top_k: usize, crp_mode: CrpMode) -> String {
    let search_start = std::time::Instant::now();

    let root = Path::new(path);
    if !root.exists() {
        return format!("ERR: path does not exist: {path}");
    }

    let root = if root.is_file() {
        root.parent().unwrap_or(root)
    } else {
        root
    };

    let index = match BM25Index::load(root) {
        Some(idx) if idx.doc_count > 0 => idx,
        _ => {
            let idx = BM25Index::build_from_directory(root);
            if idx.doc_count == 0 {
                return "No code files found to index.".to_string();
            }
            let _ = idx.save(root);
            idx
        }
    };

    let compact = crp_mode.is_tdd();
    let config = HybridConfig::default();

    let (results, search_mode) = run_hybrid_search(query, &index, root, top_k, &config);

    let search_us = search_start.elapsed().as_micros() as u64;
    global_metrics().record_search(search_us, results.len() as u64);

    let header = if compact {
        format!(
            "semantic_search({top_k}, {search_mode}) → {} results, {} chunks indexed\n",
            results.len(),
            index.doc_count
        )
    } else {
        format!(
            "Semantic search [{}]: \"{}\" ({} results from {} indexed chunks)\n",
            search_mode,
            truncate_query(query, 60),
            results.len(),
            index.doc_count,
        )
    };

    format!("{header}{}", format_hybrid_results(&results, compact))
}

pub fn handle_reindex(path: &str) -> String {
    let root = Path::new(path);
    if !root.exists() {
        return format!("ERR: path does not exist: {path}");
    }
    let root = if root.is_file() {
        root.parent().unwrap_or(root)
    } else {
        root
    };

    let idx = BM25Index::build_from_directory(root);
    let count = idx.doc_count;
    let chunks = idx.chunks.len();
    let _ = idx.save(root);

    let embed_status = try_build_embeddings(&idx, root);

    format!("Reindexed {path}: {count} files, {chunks} chunks{embed_status}")
}

fn run_hybrid_search(
    query: &str,
    index: &BM25Index,
    #[allow(unused_variables)] root: &Path,
    top_k: usize,
    #[allow(unused_variables)] config: &HybridConfig,
) -> (Vec<HybridResult>, &'static str) {
    #[cfg(feature = "embeddings")]
    {
        use crate::core::embeddings::EmbeddingEngine;

        if EmbeddingEngine::is_available() {
            if let Some(embed_idx) = EmbeddingIndex::load(root) {
                if let Some(embeddings) = embed_idx.get_aligned_embeddings(&index.chunks) {
                    if let Ok(engine) = EmbeddingEngine::load_default() {
                        let results = crate::core::hybrid_search::hybrid_search(
                            query,
                            index,
                            Some(&engine),
                            Some(&embeddings),
                            top_k,
                            config,
                        );
                        return (results, "hybrid");
                    }
                }
            }
        }
    }

    let bm25_results = index.search(query, top_k);
    let results = bm25_results
        .into_iter()
        .map(|r| HybridResult::from_bm25_public(r))
        .collect();
    (results, "bm25")
}

fn try_build_embeddings(index: &BM25Index, root: &Path) -> String {
    #[cfg(feature = "embeddings")]
    {
        use crate::core::embeddings::EmbeddingEngine;

        if !EmbeddingEngine::is_available() {
            return String::new();
        }

        let engine = match EmbeddingEngine::load_default() {
            Ok(e) => e,
            Err(_) => return String::new(),
        };

        let mut embed_idx = EmbeddingIndex::load_or_new(root, engine.dimensions());
        let changed = embed_idx.files_needing_update(&index.chunks);
        if changed.is_empty() {
            return format!(" | embeddings: {} up-to-date", embed_idx.entries.len());
        }

        let embed_start = std::time::Instant::now();
        let mut new_embeddings = Vec::new();
        for (i, chunk) in index.chunks.iter().enumerate() {
            if changed.contains(&chunk.file_path) {
                if let Ok(emb) = engine.embed(&chunk.content) {
                    new_embeddings.push((i, emb));
                }
            }
        }
        embed_idx.update(&index.chunks, &new_embeddings, &changed);
        let _ = embed_idx.save(root);

        let embed_us = embed_start.elapsed().as_micros() as u64;
        global_metrics().record_embedding(embed_us, new_embeddings.len() as u64);

        return format!(
            " | embeddings: {} updated, {} total",
            new_embeddings.len(),
            embed_idx.entries.len()
        );
    }

    #[cfg(not(feature = "embeddings"))]
    {
        let _ = (index, root);
        String::new()
    }
}

fn truncate_query(q: &str, max: usize) -> &str {
    if q.len() <= max {
        q
    } else {
        &q[..max]
    }
}
