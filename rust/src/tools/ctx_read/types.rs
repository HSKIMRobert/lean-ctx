use super::{CrpMode, ReadMode};

/// Pre-counted read output carrying the output string, resolved mode,
/// and token count computed during mode processing.
pub struct ReadOutput {
    pub content: String,
    pub resolved_mode: String,
    /// Approximate output token count from mode processing.
    /// The dispatch layer recounts the final assembled string for accurate savings.
    pub output_tokens: usize,
    /// Structurally determined cache-hit flag (#1133). Set by the code that
    /// serves the read (stub, delta, cached mode), not sniffed from rendered
    /// output. Replaces the fragile `content.contains("[unchanged")` checks.
    pub is_cache_hit: bool,
}

/// SSOT via [`ReadMode`] (#528): the `map`/`signatures` summaries whose rendered
/// body is stored per-file in `compressed_outputs`. Unknown modes are not
/// cacheable, matching the prior `["map","signatures"].contains(mode)`.
pub(crate) fn is_cacheable_mode(mode: &str) -> bool {
    mode.parse::<ReadMode>()
        .is_ok_and(|m| m.is_compressed_cacheable())
}

/// `#361` anti-inflation capping applies to whole-file views (`full` and the
/// lossy summaries `map`/`signatures`/`aggressive`/`entropy`/`task`/…), where the
/// raw file is a strict superset of the information and is therefore never a
/// worse answer when the framing happens to inflate on a small file. `full` is
/// included: an `auto` read can resolve to `full` and reach this path, and its
/// header must not push the cost above raw. Selection and delta views have
/// view-specific semantics — `lines:` returns a window, `reference` a pointer,
/// `diff` a delta, `raw` the bytes — so replacing them with the whole file would
/// be wrong, not cheaper, and they are never capped.
pub(crate) fn mode_allows_raw_cap(mode: &str) -> bool {
    // SSOT via [`ReadMode`] (#528). Unknown modes keep the prior default of
    // `true` (only `lines:`/`reference`/`diff`/`raw` opt out of the #361 cap).
    mode.parse::<ReadMode>()
        .map_or(true, |m| m.allows_raw_cap())
}

pub(crate) fn compressed_cache_key(
    mode: &str,
    crp_mode: CrpMode,
    task: Option<&str>,
    aggressiveness: Option<f64>,
    protect: &[String],
) -> String {
    // Bump when the rendered map/signatures body changes shape so stale
    // pre-line-range entries are not served from an older session cache.
    let versioned_mode = match mode {
        "map" => "map:v2",
        "signatures" => "signatures:v2",
        _ => mode,
    };
    let base = if crp_mode.is_tdd() {
        format!("{versioned_mode}:tdd")
    } else {
        versioned_mode.to_string()
    };
    // map/signatures output now embeds a task-relevant body, so task-aware and
    // task-free variants must cache under distinct keys.
    let keyed = match task.map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            t.hash(&mut h);
            format!("{base}:t{:x}", h.finish())
        }
        None => base,
    };
    // Aggressiveness and the explicit protect list both change lossy output, so
    // both must change the key (#498). Empty fragments keep pre-feature keys
    // byte-identical, so unmodified reads still hit their existing cache entries.
    let mut key = keyed;
    let aggr_frag = crate::core::aggressiveness::cache_fragment(aggressiveness);
    if !aggr_frag.is_empty() {
        key = format!("{key}:{aggr_frag}");
    }
    let protect_frag = crate::core::protect::protect_fragment(protect);
    if !protect_frag.is_empty() {
        key = format!("{key}:{protect_frag}");
    }
    key
}

/// Appends the reactive recovery footer to a compressed view, leading with the
/// MCP-free "read the path directly" route. Tier (`off|minimal|full`) and wording
/// are resolved centrally in [`crate::core::recovery`] so `ctx_read`, the shell
/// tee and archive handles all speak the same grammar. Only lossy/compressed
/// modes reach this helper, so the footer is naturally absent from `full`/`raw`.
pub(super) fn append_compressed_hint(output: &str, file_path: &str) -> String {
    match crate::core::recovery::read_footer(file_path) {
        Some(footer) => format!("{output}\n{footer}"),
        None => output.to_string(),
    }
}
