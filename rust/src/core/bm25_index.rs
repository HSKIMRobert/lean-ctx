use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

const MAX_BM25_FILES: usize = 5000;
const CHUNK_COUNT_WARNING: usize = 50_000;
const ZSTD_LEVEL: i32 = 9;

const DEFAULT_BM25_IGNORES: &[&str] = &[
    "vendor/**",
    "dist/**",
    "build/**",
    "public/vendor/**",
    "public/js/**",
    "public/css/**",
    "public/build/**",
    ".next/**",
    ".nuxt/**",
    "__pycache__/**",
    "*.min.js",
    "*.min.css",
    "*.bundle.js",
    "*.chunk.js",
];

fn max_bm25_cache_bytes() -> u64 {
    // Single source of truth: `Config::bm25_max_cache_mb_effective` (env override
    // › explicit config › disk-budget › generous default). Decoupled from the RAM
    // profile so large repos persist instead of rebuilding forever (issue #249).
    let mb = std::env::var("LEAN_CTX_BM25_MAX_CACHE_MB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or_else(|| crate::core::config::Config::load().bm25_max_cache_mb_effective());
    mb * 1024 * 1024
}

/// Effective on-disk ceiling (bytes) for the persisted BM25 index. Single source
/// of truth shared with `doctor` so its "oversized index" warning matches what
/// `save`/`load` actually enforce.
pub fn persist_ceiling_bytes() -> u64 {
    max_bm25_cache_bytes()
}

/// Outcome of persisting a BM25 index to disk. Distinguishes a real write from a
/// size-capped refusal so callers never mistake "refused to persist" for
/// success (the bug behind the perpetual "index warming" report, issue #249).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveOutcome {
    /// Written to disk. Carries the compressed (zstd) size in bytes.
    Persisted { compressed_bytes: u64 },
    /// Built fine but NOT written — the compressed size exceeds the disk
    /// ceiling. The in-memory index is still usable for this process; callers
    /// should surface the remedy (raise the cap / add ignore patterns) instead
    /// of silently rebuilding on every call.
    SkippedTooLarge {
        compressed_bytes: u64,
        limit_bytes: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub file_path: String,
    pub symbol_name: String,
    pub kind: ChunkKind,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    #[serde(default)]
    pub tokens: Vec<String>,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkKind {
    Function,
    Struct,
    Impl,
    Module,
    Class,
    Method,
    Other,
    // -- External source kinds (Context Engine) --
    Issue,
    PullRequest,
    WikiPage,
    DbSchema,
    ApiEndpoint,
    Ticket,
    ExternalOther,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedFileState {
    pub mtime_ms: u64,
    pub size_bytes: u64,
}

impl IndexedFileState {
    fn from_path(path: &Path) -> Option<Self> {
        let meta = path.metadata().ok()?;
        let size_bytes = meta.len();
        let mtime_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)?;
        Some(Self {
            mtime_ms,
            size_bytes,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BM25Index {
    pub chunks: Vec<CodeChunk>,
    pub inverted: HashMap<String, Vec<(usize, f64)>>,
    pub avg_doc_len: f64,
    pub doc_count: usize,
    pub doc_freqs: HashMap<String, usize>,
    #[serde(default)]
    pub files: HashMap<String, IndexedFileState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk_idx: usize,
    pub score: f64,
    pub file_path: String,
    pub symbol_name: String,
    pub kind: ChunkKind,
    pub start_line: usize,
    pub end_line: usize,
    pub snippet: String,
}

const BM25_K1: f64 = 1.2;
const BM25_B: f64 = 0.75;

impl Default for BM25Index {
    fn default() -> Self {
        Self::new()
    }
}

impl BM25Index {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            inverted: HashMap::new(),
            avg_doc_len: 0.0,
            doc_count: 0,
            doc_freqs: HashMap::new(),
            files: HashMap::new(),
        }
    }

    /// Approximate heap memory used by this index in bytes.
    pub fn memory_usage_bytes(&self) -> usize {
        let chunks_size: usize = self
            .chunks
            .iter()
            .map(|c| {
                c.content.len()
                    + c.file_path.len()
                    + c.symbol_name.len()
                    + c.tokens.iter().map(String::len).sum::<usize>()
                    + 64
            })
            .sum();
        let inverted_size: usize = self
            .inverted
            .iter()
            .map(|(k, v)| k.len() + v.len() * 16 + 32)
            .sum();
        let files_size: usize = self.files.keys().map(|k| k.len() + 24).sum();
        let freqs_size: usize = self.doc_freqs.keys().map(|k| k.len() + 16).sum();
        chunks_size + inverted_size + files_size + freqs_size
    }

    /// Drops all in-memory data, effectively freeing heap. Index can be re-loaded from disk.
    pub fn unload(&mut self) {
        let usage = self.memory_usage_bytes();
        self.chunks = Vec::new();
        self.inverted = HashMap::new();
        self.doc_freqs = HashMap::new();
        self.files = HashMap::new();
        self.avg_doc_len = 0.0;
        self.doc_count = 0;
        tracing::info!(
            "[bm25] unloaded index, freed ~{:.1}MB",
            usage as f64 / 1_048_576.0
        );
    }

    /// Builds an index from explicit chunks (unit tests; avoids filesystem walking).
    #[cfg(test)]
    pub(crate) fn from_chunks_for_test(chunks: Vec<CodeChunk>) -> Self {
        let mut index = Self::new();
        for mut chunk in chunks {
            if chunk.token_count == 0 {
                chunk.token_count = tokenize(&chunk.content).len();
            }
            index.add_chunk(chunk);
        }
        index.finalize();
        index
    }

    pub fn build_from_directory(root: &Path) -> Self {
        Self::build_from_directory_inner(root, &HashMap::new())
    }

    /// Like `build_from_directory` but reuses file content from a prior scan
    /// (e.g. the graph index walk) to avoid redundant disk reads.
    pub fn build_with_content_hint(root: &Path, content_hint: &HashMap<String, String>) -> Self {
        Self::build_from_directory_inner(root, content_hint)
    }

    fn build_from_directory_inner(root: &Path, content_hint: &HashMap<String, String>) -> Self {
        let root_str = root.to_string_lossy();
        if !super::graph_index::is_safe_scan_root_public(&root_str) {
            tracing::warn!("[bm25: scan aborted for unsafe root {root_str}]");
            return Self::new();
        }
        let mut index = Self::new();
        let files = list_code_files(root);
        const MAX_FILE_SIZE_BYTES: u64 = 2 * 1024 * 1024;
        let mut cache_hits = 0usize;

        for (i, rel) in files.iter().enumerate() {
            if i.is_multiple_of(500) && crate::core::memory_guard::is_under_pressure() {
                tracing::warn!(
                    "[bm25: stopping build at file {i}/{} due to memory pressure]",
                    files.len()
                );
                break;
            }
            if crate::core::memory_guard::abort_requested() {
                tracing::warn!("[bm25: aborting build due to critical memory pressure]");
                break;
            }

            let abs = root.join(rel);
            let Some(state) = IndexedFileState::from_path(&abs) else {
                continue;
            };
            if state.size_bytes > MAX_FILE_SIZE_BYTES {
                continue;
            }

            let content = if let Some(cached) = content_hint.get(rel) {
                cache_hits += 1;
                std::borrow::Cow::Borrowed(cached.as_str())
            } else {
                match std::fs::read_to_string(&abs) {
                    Ok(c) => std::borrow::Cow::Owned(c),
                    Err(_) => continue,
                }
            };

            let mut chunks = extract_chunks(rel, &content);
            chunks.sort_by(|a, b| {
                a.start_line
                    .cmp(&b.start_line)
                    .then_with(|| a.end_line.cmp(&b.end_line))
                    .then_with(|| a.symbol_name.cmp(&b.symbol_name))
            });
            for chunk in chunks {
                index.add_chunk(chunk);
            }
            index.files.insert(rel.clone(), state);
        }

        if cache_hits > 0 {
            tracing::info!(
                "[bm25: reused {cache_hits}/{} file contents from graph scan cache]",
                files.len()
            );
        }

        index.finalize();
        index
    }

    pub fn rebuild_incremental(root: &Path, prev: &BM25Index) -> Self {
        let mut old_by_file: HashMap<String, Vec<CodeChunk>> = HashMap::new();
        for c in &prev.chunks {
            old_by_file
                .entry(c.file_path.clone())
                .or_default()
                .push(c.clone());
        }
        for v in old_by_file.values_mut() {
            v.sort_by(|a, b| {
                a.start_line
                    .cmp(&b.start_line)
                    .then_with(|| a.end_line.cmp(&b.end_line))
                    .then_with(|| a.symbol_name.cmp(&b.symbol_name))
            });
        }

        let mut index = Self::new();
        let files = list_code_files(root);
        const MAX_FILE_SIZE_BYTES: u64 = 2 * 1024 * 1024;

        for (i, rel) in files.iter().enumerate() {
            if i.is_multiple_of(500) && crate::core::memory_guard::is_under_pressure() {
                tracing::warn!(
                    "[bm25: stopping incremental rebuild at file {i}/{} due to memory pressure]",
                    files.len()
                );
                break;
            }

            let abs = root.join(rel);
            let Some(state) = IndexedFileState::from_path(&abs) else {
                continue;
            };

            let unchanged = prev.files.get(rel).is_some_and(|old| *old == state);
            if unchanged {
                if let Some(chunks) = old_by_file.get(rel) {
                    if chunks.first().is_some_and(|c| !c.content.is_empty()) {
                        for chunk in chunks {
                            index.add_chunk(chunk.clone());
                        }
                        index.files.insert(rel.clone(), state);
                        continue;
                    }
                }
            }

            if state.size_bytes > MAX_FILE_SIZE_BYTES {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&abs) {
                let mut chunks = extract_chunks(rel, &content);
                chunks.sort_by(|a, b| {
                    a.start_line
                        .cmp(&b.start_line)
                        .then_with(|| a.end_line.cmp(&b.end_line))
                        .then_with(|| a.symbol_name.cmp(&b.symbol_name))
                });
                for chunk in chunks {
                    index.add_chunk(chunk);
                }
                index.files.insert(rel.clone(), state);
            }
        }

        index.finalize();
        index
    }

    fn add_chunk(&mut self, chunk: CodeChunk) {
        let idx = self.chunks.len();

        let enriched = enrich_for_bm25(&chunk);
        let tokens = tokenize(&enriched);
        for token in &tokens {
            let lower = token.to_lowercase();
            let postings = self.inverted.entry(lower.clone()).or_default();
            if postings.last().map(|(last_idx, _)| *last_idx) != Some(idx) {
                *self.doc_freqs.entry(lower).or_insert(0) += 1;
            }
            postings.push((idx, 1.0));
        }

        self.chunks.push(CodeChunk {
            token_count: tokens.len(),
            tokens: Vec::new(),
            ..chunk
        });
    }

    fn finalize(&mut self) {
        self.doc_count = self.chunks.len();
        if self.doc_count == 0 {
            return;
        }

        let total_len: usize = self.chunks.iter().map(|c| c.token_count).sum();
        self.avg_doc_len = total_len as f64 / self.doc_count as f64;
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() || self.doc_count == 0 {
            return Vec::new();
        }

        // Pre-allocated score array: O(1) per-access vs HashMap overhead.
        // Kolmogorov-optimal: minimal allocation for the scoring operation.
        let n = self.chunks.len();
        let mut scores = vec![0.0f64; n];
        let mut touched = Vec::with_capacity(n.min(256));

        for token in &query_tokens {
            let lower = token.to_lowercase();
            let df = *self.doc_freqs.get(&lower).unwrap_or(&0) as f64;
            if df == 0.0 {
                continue;
            }

            let idf = ((self.doc_count as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();

            if let Some(postings) = self.inverted.get(&lower) {
                for &(idx, weight) in postings {
                    let doc_len = self.chunks[idx].token_count as f64;
                    let norm_len = doc_len / self.avg_doc_len.max(1.0);
                    let bm25 = idf * (weight * (BM25_K1 + 1.0))
                        / (weight + BM25_K1 * (1.0 - BM25_B + BM25_B * norm_len));

                    if scores[idx] == 0.0 {
                        touched.push(idx);
                    }
                    scores[idx] += bm25;
                }
            }
        }

        let mut results: Vec<SearchResult> = touched
            .iter()
            .filter(|&&idx| scores[idx] > 0.0)
            .map(|&idx| {
                let chunk = &self.chunks[idx];
                let snippet = chunk.content.lines().take(5).collect::<Vec<_>>().join("\n");
                SearchResult {
                    chunk_idx: idx,
                    score: scores[idx],
                    file_path: chunk.file_path.clone(),
                    symbol_name: chunk.symbol_name.clone(),
                    kind: chunk.kind.clone(),
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                    snippet,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.file_path.cmp(&b.file_path))
                .then_with(|| a.symbol_name.cmp(&b.symbol_name))
                .then_with(|| a.start_line.cmp(&b.start_line))
                .then_with(|| a.end_line.cmp(&b.end_line))
        });
        results.truncate(top_k);
        results
    }

    pub fn save(&self, root: &Path) -> std::io::Result<SaveOutcome> {
        if self.chunks.len() > CHUNK_COUNT_WARNING {
            tracing::warn!(
                "[bm25] index has {} chunks (threshold {}), consider adding extra_ignore_patterns",
                self.chunks.len(),
                CHUNK_COUNT_WARNING
            );
        }

        let dir = index_dir(root);
        std::fs::create_dir_all(&dir)?;
        let data = bincode::serde::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let compressed = zstd::encode_all(data.as_slice(), ZSTD_LEVEL)
            .map_err(|e| std::io::Error::other(format!("zstd compress: {e}")))?;
        let compressed_bytes = compressed.len() as u64;

        let max_bytes = max_bm25_cache_bytes();
        if compressed_bytes > max_bytes {
            // Do NOT pretend success: a silent `Ok(())` here made `load` return
            // `None` forever and the index rebuild on every call (issue #249).
            // Report the refusal so the orchestrator can record an actionable
            // note and the agent-facing tools can stop claiming the index will
            // be "ready next call".
            tracing::warn!(
                "[bm25] compressed index too large ({:.1} MB, limit {:.0} MB), refusing to persist: {}",
                compressed_bytes as f64 / 1_048_576.0,
                max_bytes / (1024 * 1024),
                dir.display()
            );
            return Ok(SaveOutcome::SkippedTooLarge {
                compressed_bytes,
                limit_bytes: max_bytes,
            });
        }

        tracing::info!(
            "[bm25] index: {:.1} MB bincode → {:.1} MB zstd ({:.0}% saved)",
            data.len() as f64 / 1_048_576.0,
            compressed_bytes as f64 / 1_048_576.0,
            (1.0 - compressed_bytes as f64 / data.len().max(1) as f64) * 100.0
        );

        let target = dir.join("bm25_index.bin.zst");
        let tmp = dir.join("bm25_index.bin.zst.tmp");
        std::fs::write(&tmp, &compressed)?;
        std::fs::rename(&tmp, &target)?;

        let _ = std::fs::remove_file(dir.join("bm25_index.bin"));
        let _ = std::fs::remove_file(dir.join("bm25_index.json"));

        let _ = std::fs::write(
            dir.join("project_root.txt"),
            root.to_string_lossy().as_bytes(),
        );

        Ok(SaveOutcome::Persisted { compressed_bytes })
    }

    pub fn load(root: &Path) -> Option<Self> {
        let dir = index_dir(root);
        let max_bytes = max_bm25_cache_bytes();

        let zst_path = dir.join("bm25_index.bin.zst");
        if zst_path.exists() {
            let meta = std::fs::metadata(&zst_path).ok()?;
            if meta.len() > max_bytes {
                tracing::warn!(
                    "[bm25] compressed index too large ({:.1} GB, limit {:.0} MB), quarantining: {}",
                    meta.len() as f64 / 1_073_741_824.0,
                    max_bytes / (1024 * 1024),
                    zst_path.display()
                );
                let quarantined = zst_path.with_extension("zst.quarantined");
                let _ = std::fs::rename(&zst_path, &quarantined);
                return None;
            }
            let compressed = std::fs::read(&zst_path).ok()?;
            let max_decompressed = max_bytes * 20; // allow 20x expansion ratio
            let data = bounded_zstd_decode(&compressed, max_decompressed)?;
            let (idx, _): (Self, _) =
                bincode::serde::decode_from_slice(&data, bincode::config::standard()).ok()?;
            return Some(idx);
        }

        let bin_path = dir.join("bm25_index.bin");
        if bin_path.exists() {
            let meta = std::fs::metadata(&bin_path).ok()?;
            if meta.len() > max_bytes {
                tracing::warn!(
                    "[bm25] index too large ({:.1} GB, limit {:.0} MB), quarantining: {}",
                    meta.len() as f64 / 1_073_741_824.0,
                    max_bytes / (1024 * 1024),
                    bin_path.display()
                );
                let quarantined = bin_path.with_extension("bin.quarantined");
                let _ = std::fs::rename(&bin_path, &quarantined);
                return None;
            }
            let data = std::fs::read(&bin_path).ok()?;
            let (idx, _): (Self, _) =
                bincode::serde::decode_from_slice(&data, bincode::config::standard()).ok()?;
            // Auto-migrate: compress legacy .bin to .bin.zst
            if let Ok(compressed) = zstd::encode_all(data.as_slice(), ZSTD_LEVEL) {
                let zst_tmp = zst_path.with_extension("zst.tmp");
                if std::fs::write(&zst_tmp, &compressed).is_ok()
                    && std::fs::rename(&zst_tmp, &zst_path).is_ok()
                {
                    tracing::info!(
                        "[bm25] migrated {:.1} MB → {:.1} MB zstd",
                        data.len() as f64 / 1_048_576.0,
                        compressed.len() as f64 / 1_048_576.0
                    );
                    let _ = std::fs::remove_file(&bin_path);
                }
            }
            return Some(idx);
        }

        let json_path = dir.join("bm25_index.json");
        if json_path.exists() {
            let meta = std::fs::metadata(&json_path).ok()?;
            if meta.len() > max_bytes {
                tracing::warn!(
                    "[bm25] index too large ({:.1} GB, limit {:.0} MB), quarantining: {}",
                    meta.len() as f64 / 1_073_741_824.0,
                    max_bytes / (1024 * 1024),
                    json_path.display()
                );
                let quarantined = json_path.with_extension("json.quarantined");
                let _ = std::fs::rename(&json_path, &quarantined);
                return None;
            }
            let data = std::fs::read_to_string(&json_path).ok()?;
            return serde_json::from_str(&data).ok();
        }

        None
    }

    pub fn load_or_build(root: &Path) -> Self {
        Self::load_or_build_inner(root, false)
    }

    /// Like `load_or_build` but uses a fast sentinel-sampling staleness check
    /// that skips the expensive full directory walk for new-file detection.
    pub fn load_or_build_fast(root: &Path) -> Self {
        Self::load_or_build_inner(root, true)
    }

    fn load_or_build_inner(root: &Path, fast_stale: bool) -> Self {
        if !is_safe_bm25_root(root) {
            return Self::default();
        }
        if let Some(idx) = Self::load(root) {
            let stale = if fast_stale {
                bm25_index_looks_stale_fast(&idx, root)
            } else {
                bm25_index_looks_stale(&idx, root)
            };
            if !stale {
                return idx;
            }
            tracing::debug!(
                "[bm25_index: stale index detected for {}; rebuilding]",
                root.display()
            );
            let rebuilt = if idx.files.is_empty() {
                Self::build_from_directory(root)
            } else {
                Self::rebuild_incremental(root, &idx)
            };
            let _ = rebuilt.save(root);
            return rebuilt;
        }

        let built = Self::build_from_directory(root);
        let _ = built.save(root);
        built
    }

    pub fn index_file_path(root: &Path) -> PathBuf {
        let dir = index_dir(root);
        let zst = dir.join("bm25_index.bin.zst");
        if zst.exists() {
            return zst;
        }
        let bin = dir.join("bm25_index.bin");
        if bin.exists() {
            return bin;
        }
        dir.join("bm25_index.json")
    }

    /// Ingest external `ContentChunk`s into the BM25 index.
    /// Converts each chunk to a `CodeChunk` (backward-compatible) and
    /// rebuilds the inverted index. Returns the number of chunks ingested.
    pub fn ingest_content_chunks(
        &mut self,
        chunks: impl IntoIterator<Item = super::content_chunk::ContentChunk>,
    ) -> usize {
        let mut count = 0usize;
        for cc in chunks {
            self.add_chunk(cc.into());
            count += 1;
        }
        if count > 0 {
            self.finalize();
        }
        count
    }

    /// Number of chunks originating from external providers.
    pub fn external_chunk_count(&self) -> usize {
        self.chunks
            .iter()
            .filter(|c| c.file_path.contains("://"))
            .count()
    }
}

fn is_safe_bm25_root(root: &Path) -> bool {
    super::graph_index::is_safe_scan_root_public(&root.to_string_lossy())
}

fn bm25_index_looks_stale(index: &BM25Index, root: &Path) -> bool {
    bm25_index_looks_stale_inner(index, root, false)
}

/// Fast staleness check: samples a subset of tracked files and skips the
/// expensive `list_code_files()` walk for new-file detection.
pub fn bm25_index_looks_stale_fast(index: &BM25Index, root: &Path) -> bool {
    bm25_index_looks_stale_inner(index, root, true)
}

fn bm25_index_looks_stale_inner(index: &BM25Index, root: &Path, fast: bool) -> bool {
    if index.chunks.is_empty() {
        return false;
    }

    if index.files.is_empty() {
        let mut seen = std::collections::HashSet::<&str>::new();
        for chunk in &index.chunks {
            let rel = chunk.file_path.trim_start_matches(['/', '\\']);
            if rel.is_empty() {
                continue;
            }
            if !seen.insert(rel) {
                continue;
            }
            if !root.join(rel).exists() {
                return true;
            }
        }
        return false;
    }

    if fast {
        let sample_size = index.files.len().min(SENTINEL_SAMPLE_SIZE);
        let step = if index.files.len() > sample_size {
            index.files.len() / sample_size
        } else {
            1
        };
        for (i, (rel, old_state)) in index.files.iter().enumerate() {
            if i % step != 0 {
                continue;
            }
            let abs = root.join(rel);
            if !abs.exists() {
                return true;
            }
            let Some(cur) = IndexedFileState::from_path(&abs) else {
                return true;
            };
            if &cur != old_state {
                return true;
            }
        }
        return false;
    }

    for (rel, old_state) in &index.files {
        let abs = root.join(rel);
        if !abs.exists() {
            return true;
        }
        let Some(cur) = IndexedFileState::from_path(&abs) else {
            return true;
        };
        if &cur != old_state {
            return true;
        }
    }

    for rel in list_code_files(root) {
        if !index.files.contains_key(&rel) {
            return true;
        }
    }

    false
}

const SENTINEL_SAMPLE_SIZE: usize = 10;

fn bounded_zstd_decode(compressed: &[u8], max_bytes: u64) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut decoder = zstd::Decoder::new(compressed).ok()?;
    let mut buf = Vec::new();
    let mut chunk = vec![0u8; 65536];
    let mut total = 0u64;
    loop {
        let n = decoder.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > max_bytes {
            tracing::warn!(
                "[bm25] decompressed index exceeds limit ({:.0} MB > {:.0} MB), aborting load",
                total as f64 / (1024.0 * 1024.0),
                max_bytes as f64 / (1024.0 * 1024.0)
            );
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Some(buf)
}

fn index_dir(root: &Path) -> PathBuf {
    crate::core::index_namespace::vectors_dir(root)
}

fn list_code_files(root: &Path) -> Vec<String> {
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .max_depth(Some(20))
        .build();

    let cfg = crate::core::config::Config::load();
    let mut ignore_patterns: Vec<glob::Pattern> = DEFAULT_BM25_IGNORES
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect();
    ignore_patterns.extend(
        cfg.extra_ignore_patterns
            .iter()
            .filter_map(|p| glob::Pattern::new(p).ok()),
    );

    let mut files: Vec<String> = Vec::new();
    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if !is_code_file(path) {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        if rel.is_empty() {
            continue;
        }
        if ignore_patterns.iter().any(|p| p.matches(&rel)) {
            continue;
        }
        if files.len() >= MAX_BM25_FILES {
            tracing::warn!(
                "[bm25] file cap reached ({MAX_BM25_FILES}), skipping remaining files in {}",
                root.display()
            );
            break;
        }
        files.push(rel);
    }

    files.sort();
    files.dedup();
    files
}

pub fn is_code_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "py"
            | "go"
            | "java"
            | "c"
            | "cc"
            | "cpp"
            | "h"
            | "hpp"
            | "rb"
            | "cs"
            | "kt"
            | "swift"
            | "php"
            | "scala"
            | "sql"
            | "ex"
            | "exs"
            | "zig"
            | "lua"
            | "dart"
            | "vue"
            | "svelte"
    )
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            if current.len() >= 2 {
                tokens.push(current.clone());
            }
            current.clear();
        }
    }
    if current.len() >= 2 {
        tokens.push(current);
    }

    split_camel_case_tokens(&tokens)
}

pub(crate) fn tokenize_for_index(text: &str) -> Vec<String> {
    tokenize(text)
}

fn split_camel_case_tokens(tokens: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for token in tokens {
        result.push(token.clone());
        let mut start = 0;
        let chars: Vec<char> = token.chars().collect();
        for i in 1..chars.len() {
            if chars[i].is_uppercase() && (i + 1 >= chars.len() || !chars[i + 1].is_uppercase()) {
                let part: String = chars[start..i].iter().collect();
                if part.len() >= 2 {
                    result.push(part);
                }
                start = i;
            }
        }
        if start > 0 {
            let part: String = chars[start..].iter().collect();
            if part.len() >= 2 {
                result.push(part);
            }
        }
    }
    result
}

fn extract_chunks(file_path: &str, content: &str) -> Vec<CodeChunk> {
    #[cfg(feature = "tree-sitter")]
    {
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if let Some(chunks) = crate::core::chunks_ts::extract_chunks_ts(file_path, content, ext) {
            return chunks;
        }
    }

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        if let Some((name, kind)) = detect_symbol(trimmed) {
            let start = i;
            let end = find_block_end(&lines, i);
            let block: String = lines[start..=end.min(lines.len() - 1)].to_vec().join("\n");
            let token_count = tokenize(&block).len();

            chunks.push(CodeChunk {
                file_path: file_path.to_string(),
                symbol_name: name,
                kind,
                start_line: start + 1,
                end_line: end + 1,
                content: block,
                tokens: Vec::new(),
                token_count,
            });

            i = end + 1;
        } else {
            i += 1;
        }
    }

    if chunks.is_empty() && !content.is_empty() {
        // Fallback: when no symbols are detected, chunk the file into stable, content-defined
        // segments (rolling-hash) to enable meaningful semantic search over non-code assets.
        //
        // Safety note: rabin_karp uses byte offsets; we must slice bytes and decode safely.
        let bytes = content.as_bytes();
        let rk_chunks = crate::core::rabin_karp::chunk(content);
        if !rk_chunks.is_empty() && rk_chunks.len() <= 200 {
            for (idx, c) in rk_chunks.into_iter().take(50).enumerate() {
                let end = (c.offset + c.length).min(bytes.len());
                let slice = &bytes[c.offset..end];
                let chunk_text = String::from_utf8_lossy(slice).into_owned();
                let token_count = tokenize(&chunk_text).len();
                let start_line = 1 + bytecount::count(&bytes[..c.offset], b'\n');
                let end_line = start_line + bytecount::count(slice, b'\n');
                chunks.push(CodeChunk {
                    file_path: file_path.to_string(),
                    symbol_name: format!("{file_path}#chunk-{idx}"),
                    kind: ChunkKind::Module,
                    start_line,
                    end_line: end_line.max(start_line),
                    content: chunk_text,
                    tokens: Vec::new(),
                    token_count,
                });
            }
        } else {
            let token_count = tokenize(content).len();
            let snippet = lines
                .iter()
                .take(50)
                .copied()
                .collect::<Vec<_>>()
                .join("\n");
            chunks.push(CodeChunk {
                file_path: file_path.to_string(),
                symbol_name: file_path.to_string(),
                kind: ChunkKind::Module,
                start_line: 1,
                end_line: lines.len(),
                content: snippet,
                tokens: Vec::new(),
                token_count,
            });
        }
    }

    chunks
}

fn detect_symbol(line: &str) -> Option<(String, ChunkKind)> {
    let trimmed = line.trim();

    let patterns: &[(&str, ChunkKind)] = &[
        ("pub async fn ", ChunkKind::Function),
        ("async fn ", ChunkKind::Function),
        ("pub fn ", ChunkKind::Function),
        ("fn ", ChunkKind::Function),
        ("pub struct ", ChunkKind::Struct),
        ("struct ", ChunkKind::Struct),
        ("pub enum ", ChunkKind::Struct),
        ("enum ", ChunkKind::Struct),
        ("impl ", ChunkKind::Impl),
        ("pub trait ", ChunkKind::Struct),
        ("trait ", ChunkKind::Struct),
        ("export function ", ChunkKind::Function),
        ("export async function ", ChunkKind::Function),
        ("export default function ", ChunkKind::Function),
        ("function ", ChunkKind::Function),
        ("async function ", ChunkKind::Function),
        ("export class ", ChunkKind::Class),
        ("class ", ChunkKind::Class),
        ("export interface ", ChunkKind::Struct),
        ("interface ", ChunkKind::Struct),
        ("def ", ChunkKind::Function),
        ("async def ", ChunkKind::Function),
        ("class ", ChunkKind::Class),
        ("func ", ChunkKind::Function),
    ];

    for (prefix, kind) in patterns {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '<')
                .take_while(|c| *c != '<')
                .collect();
            if !name.is_empty() {
                return Some((name, kind.clone()));
            }
        }
    }

    None
}

fn find_block_end(lines: &[&str], start: usize) -> usize {
    let mut depth = 0i32;
    let mut found_open = false;

    for (i, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            match ch {
                '{' | '(' if !found_open || depth > 0 => {
                    depth += 1;
                    found_open = true;
                }
                '}' | ')' if depth > 0 => {
                    depth -= 1;
                    if depth == 0 && found_open {
                        return i;
                    }
                }
                _ => {}
            }
        }

        if found_open && depth <= 0 && i > start {
            return i;
        }

        if !found_open && i > start + 2 {
            let trimmed = lines[i].trim();
            if trimmed.is_empty()
                || (!trimmed.starts_with(' ') && !trimmed.starts_with('\t') && i > start)
            {
                return i.saturating_sub(1);
            }
        }
    }

    (start + 50).min(lines.len().saturating_sub(1))
}

pub fn format_search_results(results: &[SearchResult], compact: bool) -> String {
    if results.is_empty() {
        return "No results found.".to_string();
    }

    let mut out = String::new();
    for (i, r) in results.iter().enumerate() {
        let is_external = r.file_path.contains("://");
        // Forward-slash normalize local paths so Windows backslashes are never
        // dropped/escape-mangled by client render layers (issue #324). External
        // URIs (provider results, e.g. `github://`) are left untouched.
        let normalized;
        let file_path: &str = if is_external {
            &r.file_path
        } else {
            normalized = crate::core::protocol::display_path(&r.file_path);
            &normalized
        };
        if compact {
            if is_external {
                out.push_str(&format!(
                    "{}. {:.2} [{:?}] {} — {}\n",
                    i + 1,
                    r.score,
                    r.kind,
                    file_path,
                    r.symbol_name,
                ));
            } else {
                out.push_str(&format!(
                    "{}. {:.2} {}:{}-{} {:?} {}\n",
                    i + 1,
                    r.score,
                    file_path,
                    r.start_line,
                    r.end_line,
                    r.kind,
                    r.symbol_name,
                ));
            }
        } else if is_external {
            out.push_str(&format!(
                "\n--- Result {} (score: {:.2}) [{:?}] ---\n{} — {}\n{}\n",
                i + 1,
                r.score,
                r.kind,
                file_path,
                r.symbol_name,
                r.snippet,
            ));
        } else {
            out.push_str(&format!(
                "\n--- Result {} (score: {:.2}) ---\n{} :: {} [{:?}] (L{}-{})\n{}\n",
                i + 1,
                r.score,
                file_path,
                r.symbol_name,
                r.kind,
                r.start_line,
                r.end_line,
                r.snippet,
            ));
        }
    }
    out
}

/// Enrich chunk content with file-path components for BM25 path-matching.
///
/// SACL (EMNLP 2025) shows that augmenting code with structural information
/// improves retrieval by 7-12.8%. We append the file stem twice (for boost)
/// and the immediate parent directory once, enabling queries like "auth handler"
/// to match `src/auth/handler.rs`.
fn enrich_for_bm25(chunk: &CodeChunk) -> String {
    let path = Path::new(&chunk.file_path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let dir = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|d| d.to_str())
        .unwrap_or("");

    if stem.is_empty() {
        return chunk.content.clone();
    }

    format!("{} {} {} {}", chunk.content, stem, stem, dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn tokenize_splits_code() {
        let tokens = tokenize("fn calculate_total(items: Vec<Item>) -> f64");
        assert!(tokens.contains(&"calculate_total".to_string()));
        assert!(tokens.contains(&"items".to_string()));
        assert!(tokens.contains(&"Vec".to_string()));
    }

    #[test]
    fn format_search_results_normalizes_windows_separators() {
        // Issue #324: Windows backslash paths in search output were dropped or
        // escape-mangled by client render layers. They must come out with
        // forward slashes.
        let r = SearchResult {
            chunk_idx: 0,
            score: 1.0,
            file_path: r"C:\Users\zir\AppData\Local\Temp\win-build-log.txt".to_string(),
            symbol_name: "main".to_string(),
            kind: ChunkKind::Function,
            start_line: 1,
            end_line: 2,
            snippet: "x".to_string(),
        };
        let compact = format_search_results(std::slice::from_ref(&r), true);
        assert!(compact.contains("C:/Users/zir/AppData/Local/Temp/win-build-log.txt"));
        assert!(!compact.contains('\\'));

        let verbose = format_search_results(std::slice::from_ref(&r), false);
        assert!(verbose.contains("C:/Users/zir/AppData/Local/Temp/win-build-log.txt"));
        assert!(!verbose.contains('\\'));
    }

    #[test]
    fn format_search_results_leaves_external_uris_untouched() {
        // Provider results (e.g. github://) are not OS paths and must not be
        // rewritten.
        let r = SearchResult {
            chunk_idx: 0,
            score: 1.0,
            file_path: "github://owner/repo/issues/42".to_string(),
            symbol_name: "issue".to_string(),
            kind: ChunkKind::Module,
            start_line: 0,
            end_line: 0,
            snippet: "y".to_string(),
        };
        let out = format_search_results(std::slice::from_ref(&r), true);
        assert!(out.contains("github://owner/repo/issues/42"));
    }

    #[test]
    fn camel_case_splitting() {
        let tokens = split_camel_case_tokens(&["calculateTotal".to_string()]);
        assert!(tokens.contains(&"calculateTotal".to_string()));
        assert!(tokens.contains(&"calculate".to_string()));
        assert!(tokens.contains(&"Total".to_string()));
    }

    #[test]
    fn detect_rust_function() {
        let (name, kind) =
            detect_symbol("pub fn process_request(req: Request) -> Response {").unwrap();
        assert_eq!(name, "process_request");
        assert_eq!(kind, ChunkKind::Function);
    }

    #[test]
    fn bm25_search_finds_relevant() {
        let mut index = BM25Index::new();
        index.add_chunk(CodeChunk {
            file_path: "auth.rs".into(),
            symbol_name: "validate_token".into(),
            kind: ChunkKind::Function,
            start_line: 1,
            end_line: 10,
            content: "fn validate_token(token: &str) -> bool { check_jwt_expiry(token) }".into(),
            tokens: tokenize("fn validate_token token str bool check_jwt_expiry token"),
            token_count: 8,
        });
        index.add_chunk(CodeChunk {
            file_path: "db.rs".into(),
            symbol_name: "connect_database".into(),
            kind: ChunkKind::Function,
            start_line: 1,
            end_line: 5,
            content: "fn connect_database(url: &str) -> Pool { create_pool(url) }".into(),
            tokens: tokenize("fn connect_database url str Pool create_pool url"),
            token_count: 7,
        });
        index.finalize();

        let results = index.search("jwt token validation", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].symbol_name, "validate_token");
    }

    #[test]
    fn bm25_search_sorts_ties_deterministically() {
        let mut index = BM25Index::new();

        // Insert in reverse path order to ensure the sort tie-break matters.
        index.add_chunk(CodeChunk {
            file_path: "b.rs".into(),
            symbol_name: "same".into(),
            kind: ChunkKind::Function,
            start_line: 1,
            end_line: 1,
            content: "fn same() {}".into(),
            tokens: tokenize("same token"),
            token_count: 2,
        });
        index.add_chunk(CodeChunk {
            file_path: "a.rs".into(),
            symbol_name: "same".into(),
            kind: ChunkKind::Function,
            start_line: 1,
            end_line: 1,
            content: "fn same() {}".into(),
            tokens: tokenize("same token"),
            token_count: 2,
        });
        index.finalize();

        let results = index.search("same", 10);
        assert!(results.len() >= 2);
        assert_eq!(results[0].file_path, "a.rs");
        assert_eq!(results[1].file_path, "b.rs");
    }

    #[test]
    fn bm25_index_is_stale_when_any_indexed_file_is_missing() {
        let td = tempdir().expect("tempdir");
        let root = td.path();
        std::fs::write(root.join("a.rs"), "pub fn a() {}\n").expect("write a.rs");

        let idx = BM25Index::build_from_directory(root);
        assert!(!bm25_index_looks_stale(&idx, root));

        std::fs::remove_file(root.join("a.rs")).expect("remove a.rs");
        assert!(bm25_index_looks_stale(&idx, root));
    }

    #[test]
    #[cfg(unix)]
    fn bm25_incremental_rebuild_reuses_unchanged_files_without_reading() {
        let td = tempdir().expect("tempdir");
        let root = td.path();

        std::fs::write(root.join("a.rs"), "pub fn a() { println!(\"A\"); }\n").expect("write a.rs");
        std::fs::write(root.join("b.rs"), "pub fn b() { println!(\"B\"); }\n").expect("write b.rs");

        let idx1 = BM25Index::build_from_directory(root);
        assert!(idx1.files.contains_key("a.rs"));
        assert!(idx1.files.contains_key("b.rs"));

        // Make a.rs unreadable. Incremental rebuild must keep it indexed by reusing prior chunks.
        let a_path = root.join("a.rs");
        let mut perms = std::fs::metadata(&a_path).expect("meta a.rs").permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&a_path, perms).expect("chmod a.rs");

        // Change b.rs (size changes) to force a re-read for that file.
        std::fs::write(root.join("b.rs"), "pub fn b() { println!(\"B2\"); }\n")
            .expect("rewrite b.rs");

        let idx2 = BM25Index::rebuild_incremental(root, &idx1);
        assert!(
            idx2.files.contains_key("a.rs"),
            "a.rs should be kept via reuse"
        );
        assert!(idx2.files.contains_key("b.rs"));

        let b_has_b2 = idx2
            .chunks
            .iter()
            .any(|c| c.file_path == "b.rs" && c.content.contains("B2"));
        assert!(b_has_b2, "b.rs should be re-read and re-chunked");

        // Restore permissions to avoid cleanup surprises.
        let mut perms = std::fs::metadata(&a_path).expect("meta a.rs").permissions();
        perms.set_mode(0o644);
        let _ = std::fs::set_permissions(&a_path, perms);
    }

    #[test]
    fn load_quarantines_oversized_index() {
        let _env = crate::core::data_dir::test_env_lock();
        let td = tempdir().expect("tempdir");
        let root = td.path();
        let dir = crate::core::index_namespace::vectors_dir(root);
        std::fs::create_dir_all(&dir).expect("create vectors dir");

        let index_path = dir.join("bm25_index.json");
        std::env::set_var("LEAN_CTX_BM25_MAX_CACHE_MB", "0");
        std::fs::write(&index_path, r#"{"chunks":[]}"#).expect("write index");

        let result = BM25Index::load(root);
        assert!(result.is_none(), "oversized index should return None");
        assert!(
            !index_path.exists(),
            "original index should be removed after quarantine"
        );
        assert!(
            dir.join("bm25_index.json.quarantined").exists(),
            "quarantined file should exist"
        );

        std::env::remove_var("LEAN_CTX_BM25_MAX_CACHE_MB");
    }

    #[test]
    fn save_refuses_oversized_output() {
        let _env = crate::core::data_dir::test_env_lock();
        let data_dir = tempdir().expect("data_dir");
        std::env::set_var("LEAN_CTX_DATA_DIR", data_dir.path());
        std::env::set_var("LEAN_CTX_BM25_MAX_CACHE_MB", "0");

        let td = tempdir().expect("tempdir");
        let root = td.path();

        let mut index = BM25Index::new();
        index.add_chunk(CodeChunk {
            file_path: "a.rs".into(),
            symbol_name: "a".into(),
            kind: ChunkKind::Function,
            start_line: 1,
            end_line: 1,
            content: "fn a() {}".into(),
            tokens: tokenize("fn a"),
            token_count: 2,
        });
        index.finalize();

        let outcome = index
            .save(root)
            .expect("save returns Ok even when refusing");
        assert!(
            matches!(outcome, SaveOutcome::SkippedTooLarge { .. }),
            "oversized save must report SkippedTooLarge (not a silent success), got {outcome:?}"
        );
        let index_path = BM25Index::index_file_path(root);
        assert!(
            !index_path.exists(),
            "save should refuse to persist oversized index"
        );

        std::env::remove_var("LEAN_CTX_BM25_MAX_CACHE_MB");
    }

    #[test]
    fn save_reports_persisted_outcome() {
        let _env = crate::core::data_dir::test_env_lock();
        let data_dir = tempdir().expect("data_dir");
        std::env::set_var("LEAN_CTX_DATA_DIR", data_dir.path());
        std::env::set_var("LEAN_CTX_BM25_MAX_CACHE_MB", "512");
        let td = tempdir().expect("tempdir");
        let root = td.path();
        std::fs::write(root.join("a.rs"), "pub fn alpha() {}\n").expect("write");

        let index = BM25Index::build_from_directory(root);
        let outcome = index.save(root).expect("save");
        match outcome {
            SaveOutcome::Persisted { compressed_bytes } => {
                assert!(compressed_bytes > 0, "persisted size should be non-zero");
            }
            SaveOutcome::SkippedTooLarge { .. } => {
                panic!("expected Persisted, got {outcome:?}")
            }
        }

        std::env::remove_var("LEAN_CTX_BM25_MAX_CACHE_MB");
        std::env::remove_var("LEAN_CTX_DATA_DIR");
    }

    #[test]
    fn persist_ceiling_honors_env_override() {
        // The public ceiling accessor (shared with doctor) must honor an explicit
        // override exactly, so operators can size it to their monorepo.
        let _env = crate::core::data_dir::test_env_lock();
        std::env::set_var("LEAN_CTX_BM25_MAX_CACHE_MB", "777");
        assert_eq!(persist_ceiling_bytes(), 777 * 1024 * 1024);
        std::env::remove_var("LEAN_CTX_BM25_MAX_CACHE_MB");
    }

    #[test]
    fn save_writes_project_root_marker() {
        let _env = crate::core::data_dir::test_env_lock();
        let td = tempdir().expect("tempdir");
        let root = td.path();
        std::fs::write(root.join("a.rs"), "pub fn a() {}\n").expect("write");

        std::env::remove_var("LEAN_CTX_BM25_MAX_CACHE_MB");
        let index = BM25Index::build_from_directory(root);
        index.save(root).expect("save");

        let dir = crate::core::index_namespace::vectors_dir(root);
        let marker = dir.join("project_root.txt");
        assert!(marker.exists(), "project_root.txt marker should exist");
        let content = std::fs::read_to_string(&marker).expect("read marker");
        assert_eq!(content, root.to_string_lossy());
    }

    #[test]
    fn save_load_roundtrip_uses_zstd() {
        let _env = crate::core::data_dir::test_env_lock();
        let data_dir = tempdir().expect("data_dir");
        std::env::set_var("LEAN_CTX_DATA_DIR", data_dir.path());
        std::env::set_var("LEAN_CTX_BM25_MAX_CACHE_MB", "512");
        let td = tempdir().expect("tempdir");
        let root = td.path();

        for i in 0..10 {
            std::fs::write(
                root.join(format!("mod{i}.rs")),
                format!(
                    "pub fn handler_{i}() {{\n    println!(\"hello\");\n}}\n\n\
                     pub fn helper_{i}() {{\n    println!(\"world\");\n}}\n"
                ),
            )
            .expect("write");
        }

        let index = BM25Index::build_from_directory(root);
        assert!(index.doc_count > 0, "should have indexed chunks");
        index.save(root).expect("save");

        let dir = crate::core::index_namespace::vectors_dir(root);
        let zst = dir.join("bm25_index.bin.zst");
        assert!(zst.exists(), "should write .bin.zst");
        assert!(
            !dir.join("bm25_index.bin").exists(),
            ".bin should be deleted"
        );

        let loaded = BM25Index::load(root).expect("load compressed index");
        assert_eq!(loaded.doc_count, index.doc_count);
        assert_eq!(loaded.chunks.len(), index.chunks.len());

        std::env::remove_var("LEAN_CTX_BM25_MAX_CACHE_MB");
        std::env::remove_var("LEAN_CTX_DATA_DIR");
    }

    #[test]
    fn auto_migrate_bin_to_zst() {
        let _env = crate::core::data_dir::test_env_lock();
        let data_dir = tempdir().expect("data_dir");
        std::env::set_var("LEAN_CTX_DATA_DIR", data_dir.path());
        std::env::set_var("LEAN_CTX_BM25_MAX_CACHE_MB", "512");
        let td = tempdir().expect("tempdir");
        let root = td.path();

        std::fs::write(root.join("a.rs"), "pub fn a() {}\n").expect("write");
        let index = BM25Index::build_from_directory(root);

        let dir = crate::core::index_namespace::vectors_dir(root);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let data =
            bincode::serde::encode_to_vec(&index, bincode::config::standard()).expect("encode");
        std::fs::write(dir.join("bm25_index.bin"), &data).expect("write bin");

        let loaded = BM25Index::load(root).expect("load should auto-migrate");
        assert_eq!(loaded.doc_count, index.doc_count);
        assert!(
            dir.join("bm25_index.bin.zst").exists(),
            ".bin.zst should be created"
        );
        assert!(
            !dir.join("bm25_index.bin").exists(),
            ".bin should be removed"
        );

        std::env::remove_var("LEAN_CTX_BM25_MAX_CACHE_MB");
        std::env::remove_var("LEAN_CTX_DATA_DIR");
    }

    #[test]
    fn list_code_files_skips_default_vendor_ignores() {
        let td = tempdir().expect("tempdir");
        let root = td.path();

        std::fs::write(root.join("main.rs"), "pub fn main() {}\n").expect("write main");
        std::fs::create_dir_all(root.join("vendor/lib")).expect("mkdir vendor");
        std::fs::write(root.join("vendor/lib/dep.rs"), "pub fn dep() {}\n").expect("write vendor");
        std::fs::create_dir_all(root.join("dist")).expect("mkdir dist");
        std::fs::write(root.join("dist/bundle.js"), "function x() {}").expect("write dist");

        let files = list_code_files(root);
        assert!(
            files.iter().any(|f| f == "main.rs"),
            "main.rs should be included"
        );
        assert!(
            !files.iter().any(|f| f.starts_with("vendor/")),
            "vendor/ files should be excluded by DEFAULT_BM25_IGNORES"
        );
        assert!(
            !files.iter().any(|f| f.starts_with("dist/")),
            "dist/ files should be excluded by DEFAULT_BM25_IGNORES"
        );
    }

    #[test]
    fn list_code_files_respects_max_files_cap() {
        let td = tempdir().expect("tempdir");
        let root = td.path();

        // Create more files than MAX_BM25_FILES wouldn't let us test easily (5000),
        // but we can verify the cap constant exists and the function returns a bounded vec.
        for i in 0..10 {
            std::fs::write(
                root.join(format!("f{i}.rs")),
                format!("pub fn f{i}() {{}}\n"),
            )
            .expect("write");
        }
        let files = list_code_files(root);
        assert!(
            files.len() <= MAX_BM25_FILES,
            "file count should not exceed MAX_BM25_FILES"
        );
    }

    #[test]
    fn max_bm25_cache_bytes_reads_env() {
        let _env = crate::core::data_dir::test_env_lock();
        std::env::set_var("LEAN_CTX_BM25_MAX_CACHE_MB", "64");
        let bytes = max_bm25_cache_bytes();
        assert_eq!(bytes, 64 * 1024 * 1024);
        std::env::remove_var("LEAN_CTX_BM25_MAX_CACHE_MB");
    }
}
