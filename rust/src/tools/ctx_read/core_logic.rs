use super::{
    CrpMode, Path, ReadOutput, ReadTuning, SessionCache, compressed_cache_key, compressor,
    count_tokens, find_similar_and_update_semantic_index, format_full_compact_output,
    format_full_output, is_cacheable_mode, mode_allows_raw_cap, process_mode_tuned, protocol,
    read_file_lossy, try_disk_anchored_window, try_stub_hit_readonly,
};

pub(super) fn handle_with_options_inner(
    cache: &mut SessionCache,
    path: &str,
    mode: &str,
    fresh: bool,
    crp_mode: CrpMode,
    task: Option<&str>,
    tuning: ReadTuning<'_>,
    preread: Option<String>,
) -> ReadOutput {
    let file_ref = cache.get_file_ref(path);
    let short = protocol::shorten_path(path);
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    // #1150: a path the operator marked "never compress" is always returned in
    // full — exact bytes matter more than token savings for these files (golden
    // snapshots, byte-asserted fixtures, security-sensitive configs). Every lossy
    // mode (auto, aggressive, signatures, density, diff, …) collapses to the
    // verbatim full read; `raw` (already verbatim) and explicit `lines:` slices
    // are left as the user asked. The default config protects nothing, so this is
    // a fast no-op for everyone who hasn't opted in.
    let mode = if mode != "raw"
        && !mode.starts_with("lines:")
        && crate::core::config::Config::load()
            .proxy
            .is_path_compress_protected(path)
    {
        "full"
    } else {
        mode
    };

    if fresh {
        if mode == "diff" {
            let warning = "[warning] fresh+diff is redundant — fresh invalidates cache, no diff possible. Use mode=full with fresh=true instead.";
            return ReadOutput {
                content: warning.to_string(),
                resolved_mode: "diff".into(),
                output_tokens: count_tokens(warning),
                is_cache_hit: false,
            };
        }
        cache.invalidate(path);
    }

    // #811: a fresh, explicitly windowed `anchored:N-M` read never needs the
    // cache (fresh always bypasses it) or the whole file in memory — try the
    // disk-streaming short-circuit first.
    if let Some(out) =
        try_disk_anchored_window(path, mode, fresh, preread.is_none(), &file_ref, &short)
    {
        return out;
    }

    if mode == "diff" {
        let (out, _) = handle_diff(cache, path, &file_ref);
        let out = crate::core::redaction::redact_text_if_enabled(&out);
        let sent = count_tokens(&out);
        return ReadOutput {
            content: out,
            resolved_mode: "diff".into(),
            output_tokens: sent,
            is_cache_hit: false,
        };
    }

    if mode != "full"
        && let Some(existing) = cache.get(path)
    {
        let stale = crate::core::cache::is_cache_entry_stale_verified(
            path,
            existing.stored_mtime,
            &existing.hash,
        );
        if stale {
            cache.invalidate(path);
        }
    }

    // Snapshot the minimal immutable data the miss paths need, then drop the
    // borrow before any mutable operations (set_compressed, invalidate, store).
    let cache_snapshot = cache
        .get(path)
        .map(|existing| (existing.original_tokens, existing.content()));

    if let Some((original_tokens, content_opt)) = cache_snapshot {
        // Resolve the read mode first — and *cache-aware* for `auto`. Handing the
        // live cache to the resolver is what lets an `auto` re-read of an
        // unchanged, already-fully-delivered file short-circuit to
        // ("full", "cache_hit") and collapse to the cheap ~13-token `[unchanged]`
        // stub, exactly like an explicit `full` re-read. The previous call passed
        // no cache, so that branch was dead code and every `auto` re-read
        // re-delivered the whole file ("re-reads aren't cached"). Resolving
        // up-front also lets us hit the compressed-output cache BEFORE
        // decompressing the full body (avoids ~2-5ms zstd on hits). The
        // aggressiveness knob (#714) still routes `auto` through the density path.
        let resolved_mode = if mode == "auto" {
            tuning
                .auto_density_mode()
                .unwrap_or_else(|| resolve_auto_mode(Some(cache), path, original_tokens, task))
        } else {
            mode.to_string()
        };

        if resolved_mode == "full" || resolved_mode == "full-compact" {
            if let Some(out) = try_stub_hit_readonly(cache, path) {
                return out;
            }
            if resolved_mode == "full-compact" {
                let content = match read_file_lossy(path) {
                    Ok(c) => c,
                    Err(e) => {
                        let msg = format!("ERROR: {e}");
                        return ReadOutput {
                            content: msg,
                            resolved_mode: "error".into(),
                            output_tokens: 0,
                            is_cache_hit: false,
                        };
                    }
                };
                let (out, _) = format_full_compact_output(&content);
                let out = crate::core::redaction::redact_text_if_enabled(&out);
                let sent = count_tokens(&out);
                return ReadOutput {
                    content: out,
                    resolved_mode: "full-compact".into(),
                    output_tokens: sent,
                    is_cache_hit: false,
                };
            }
            let (out, _) = handle_full_with_auto_delta(cache, path, &file_ref, &short, ext, task);
            let out = crate::core::redaction::redact_text_if_enabled(&out);
            let sent = count_tokens(&out);
            return ReadOutput {
                content: out,
                resolved_mode: "full".into(),
                output_tokens: sent,
                is_cache_hit: false,
            };
        }

        if is_cacheable_mode(&resolved_mode) {
            let cache_key = compressed_cache_key(
                &resolved_mode,
                crp_mode,
                task,
                tuning.aggressiveness,
                tuning.protect,
            );
            let compressed_hit = cache.get_compressed(path, &cache_key).cloned();
            if let Some(cached_output) = compressed_hit {
                // get_compressed() already recorded the cache hit (stats + event)
                let out = crate::core::redaction::redact_text_if_enabled(&cached_output);
                let sent = count_tokens(&out);
                return ReadOutput {
                    content: out,
                    resolved_mode,
                    output_tokens: sent,
                    is_cache_hit: true,
                };
            }
        }

        if let Some(content) = content_opt {
            let (out, _) = process_mode_tuned(
                &content,
                &resolved_mode,
                &file_ref,
                &short,
                ext,
                original_tokens,
                crp_mode,
                path,
                task,
                tuning,
            );
            // #361 anti-inflation for lossy whole-file summaries (auto OR
            // explicit): map/signatures/… must never cost more than the raw file.
            // Selection/delta views keep their exact shape (see
            // mode_allows_raw_cap). Cap before caching so re-read hits serve the
            // same capped, byte-stable body.
            let out = if mode_allows_raw_cap(&resolved_mode) {
                let framed_tokens = count_tokens(&out);
                cap_to_raw(out, framed_tokens, &content, original_tokens)
            } else {
                out
            };
            if is_cacheable_mode(&resolved_mode) {
                let cache_key = compressed_cache_key(
                    &resolved_mode,
                    crp_mode,
                    task,
                    tuning.aggressiveness,
                    tuning.protect,
                );
                cache.set_compressed(path, &cache_key, out.clone());
            }
            let out = crate::core::redaction::redact_text_if_enabled(&out);
            let sent = count_tokens(&out);
            return ReadOutput {
                content: out,
                resolved_mode,
                output_tokens: sent,
                is_cache_hit: false,
            };
        }
        cache.invalidate(path);
    }

    // Two-Phase Read (#1098): when pre-read content was provided (disk I/O
    // already happened outside the cache lock), use it directly. Otherwise
    // fall back to reading from disk (legacy path, still used by fast-path
    // inline calls where the write lock was immediately available).
    let content = if let Some(pr) = preread {
        pr
    } else {
        match read_file_lossy(path) {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("ERROR: {e}");
                let tokens = count_tokens(&msg);
                return ReadOutput {
                    content: msg,
                    resolved_mode: "error".into(),
                    output_tokens: tokens,
                    is_cache_hit: false,
                };
            }
        }
    };

    let store_result = cache.store(path, &content);

    // Skip expensive hint computation for line-range reads and first reads.
    // Hints are only useful from the 2nd read onwards when the file is contextually relevant.
    let is_line_range = mode.starts_with("lines:");
    let hints = crate::core::profiles::active_profile().output_hints;
    let is_repeat_read = store_result.read_count > 1;
    let similar_hint = if !is_line_range && is_repeat_read && hints.semantic_hint() {
        find_similar_and_update_semantic_index(path, &content)
    } else {
        None
    };
    // #1098: graph hints moved to background — `graph_related_hint()` does a
    // SQLite query that can block for 50-200ms on Windows, which is unacceptable
    // while holding the global cache write-lock. The registered handler calls it
    // after releasing the lock and appends it to the response.
    let graph_hint: Option<String> = None;

    if mode == "full" || mode == "full-compact" {
        cache.mark_full_delivered(path);

        if mode == "full-compact" {
            let (output, _) = format_full_compact_output(&content);
            let output = crate::core::redaction::redact_text_if_enabled(&output);
            let sent = count_tokens(&output);
            return ReadOutput {
                content: output,
                resolved_mode: "full-compact".into(),
                output_tokens: sent,
                is_cache_hit: false,
            };
        }

        let (mut output, _) = format_full_output(
            &file_ref,
            &short,
            ext,
            &content,
            store_result.original_tokens,
            store_result.line_count,
            task,
        );
        if let Some(hint) = &graph_hint {
            output.push_str(&format!("\n{hint}"));
        }
        if let Some(hint) = similar_hint {
            output.push_str(&format!("\n{hint}"));
        }
        let framed_tokens = count_tokens(&output);
        let output = cap_to_raw(
            output,
            framed_tokens,
            &content,
            store_result.original_tokens,
        );
        let output = crate::core::redaction::redact_text_if_enabled(&output);
        let sent = count_tokens(&output);
        return ReadOutput {
            content: output,
            resolved_mode: "full".into(),
            output_tokens: sent,
            is_cache_hit: false,
        };
    }

    let resolved_mode = if mode == "auto" {
        tuning
            .auto_density_mode()
            .unwrap_or_else(|| resolve_auto_mode(None, path, store_result.original_tokens, task))
    } else {
        mode.to_string()
    };

    let (output, _sent) = process_mode_tuned(
        &content,
        &resolved_mode,
        &file_ref,
        &short,
        ext,
        store_result.original_tokens,
        crp_mode,
        path,
        task,
        tuning,
    );
    // #361 anti-inflation for lossy whole-file summaries (auto OR explicit);
    // selection/delta views keep their exact shape (see mode_allows_raw_cap).
    // Cap first, then cache the pure capped body so re-reads stay byte-stable
    // (#498) — the optional, read-state-dependent navigation hints below are
    // appended to the returned value only, never to the cached body.
    let mut output = if mode_allows_raw_cap(&resolved_mode) {
        let framed_tokens = count_tokens(&output);
        cap_to_raw(
            output,
            framed_tokens,
            &content,
            store_result.original_tokens,
        )
    } else {
        output
    };
    if is_cacheable_mode(&resolved_mode) {
        let cache_key = compressed_cache_key(
            &resolved_mode,
            crp_mode,
            task,
            tuning.aggressiveness,
            tuning.protect,
        );
        cache.set_compressed(path, &cache_key, output.clone());
    }
    if let Some(hint) = &graph_hint {
        output.push_str(&format!("\n{hint}"));
    }
    if let Some(hint) = similar_hint {
        output.push_str(&format!("\n{hint}"));
    }
    let output = crate::core::redaction::redact_text_if_enabled(&output);
    let final_tokens = count_tokens(&output);
    ReadOutput {
        content: output,
        resolved_mode,
        output_tokens: final_tokens,
        is_cache_hit: false,
    }
}

/// #361 anti-inflation invariant: a `ctx_read` must never cost more tokens than
/// reading the raw file would. Framing (file-ref header, deps/exports summary,
/// savings footer, navigation hints) only earns its keep on large files and
/// repeated reads — on a cold read of a small file it is pure overhead, the
/// exact inflation an independent benchmark measured (#361). When the framed
/// payload exceeds the bare content we ship the content verbatim, so a read is
/// break-even at worst and a win whenever a compressed mode or a cached re-read
/// applies. Re-reads are unaffected: the cache keys on path and re-derives the
/// file ref, so dropping the cold header here costs nothing on the next read.
///
/// `framed_tokens` and `raw_tokens` are both measured pre-redaction (redaction
/// is roughly token-neutral and applied to whichever string wins), so the
/// comparison is apples-to-apples with `original_tokens`. Empty files
/// (`raw_tokens == 0`) keep their framing so the reader still gets a signal.
pub(crate) fn cap_to_raw(
    framed: String,
    framed_tokens: usize,
    raw_content: &str,
    raw_tokens: usize,
) -> String {
    if raw_tokens > 0 && framed_tokens > raw_tokens {
        let prevented = (framed_tokens - raw_tokens) as u64;
        crate::core::cache_telemetry::record_raw_cap(prevented);
        raw_content.to_string()
    } else {
        framed
    }
}

/// Delegates to the unified `auto_mode_resolver::resolve()`.
/// Resolve `auto` to a concrete mode.
///
/// Pass `Some(cache)` on the warm read path: the resolver then short-circuits an
/// unchanged, already-fully-delivered file to `("full", "cache_hit")` so the
/// caller can collapse the re-read to the cheap `[unchanged]` stub instead of
/// re-delivering the whole body. Pass `None` only where no session cache exists
/// (the CLI cold path), which forces a stateless cold resolution.
pub(crate) fn resolve_auto_mode(
    cache: Option<&SessionCache>,
    file_path: &str,
    original_tokens: usize,
    task: Option<&str>,
) -> String {
    let ctx = crate::core::auto_mode_resolver::AutoModeContext {
        path: file_path,
        token_count: original_tokens,
        task,
        cache,
    };
    crate::core::auto_mode_resolver::resolve(&ctx).mode
}

const AUTO_DELTA_THRESHOLD: f64 = 0.6;

/// Re-reads from disk; if content changed and delta is compact, sends auto-delta.
pub(super) fn handle_full_with_auto_delta(
    cache: &mut SessionCache,
    path: &str,
    file_ref: &str,
    short: &str,
    ext: &str,
    task: Option<&str>,
) -> (String, usize) {
    let _mode_guard = crate::core::savings_footer::ModeGuard::new("full");
    let Ok(disk_content) = read_file_lossy(path) else {
        cache.record_cache_hit(path);
        if let Some(existing) = cache.get(path) {
            if !crate::core::protocol::meta_visible()
                && let Some(cached) = existing.content()
            {
                return format_full_output(
                    file_ref,
                    short,
                    ext,
                    &cached,
                    existing.original_tokens,
                    existing.line_count,
                    task,
                );
            }
            let out = format!(
                "[using cached version — file read failed]\n{file_ref}={short} cached {}t {}L",
                existing.read_count(),
                existing.line_count
            );
            let sent = count_tokens(&out);
            return (out, sent);
        }
        let out = if crate::core::protocol::meta_visible() && !file_ref.is_empty() {
            format!("[file read failed and no cached version available] {file_ref}={short}")
        } else {
            format!("[file read failed and no cached version available] {short}")
        };
        let sent = count_tokens(&out);
        return (out, sent);
    };

    let no_deg = crate::core::config::Config::load().no_degrade_effective();
    let prof = crate::core::profiles::active_profile();
    let force_full = no_deg
        || (prof.read.default_mode_effective() == "full"
            && prof.compression.crp_mode_effective() == "off");

    let old_content = cache
        .get(path)
        .and_then(crate::core::cache::CacheEntry::content)
        .unwrap_or_default();
    let store_result = cache.store(path, &disk_content);

    if store_result.was_hit {
        // #1128: no stub here. Whether an unchanged file may collapse to
        // `[unchanged …]` is decided once, by `try_stub_hit_readonly`, which the
        // caller already consulted before routing here — and only that gate knows
        // whether THIS conversation received the content (#954/#955). A second
        // decision built from `StoreResult` cannot: `full_content_delivered` is
        // carried over from the cache entry, so it answers "some conversation got
        // this", which is the question the gate exists to stop trusting.
        cache.mark_full_delivered(path);
        return format_full_output(
            file_ref,
            short,
            ext,
            &disk_content,
            store_result.original_tokens,
            store_result.line_count,
            task,
        );
    }

    let diff = compressor::diff_content(&old_content, &disk_content);
    let diff_tokens = count_tokens(&diff);
    let full_tokens = store_result.original_tokens;

    if !force_full
        && full_tokens > 0
        && (diff_tokens as f64) < (full_tokens as f64 * AUTO_DELTA_THRESHOLD)
    {
        let savings = protocol::format_savings(full_tokens, diff_tokens);
        let head = if crate::core::protocol::meta_visible() && !file_ref.is_empty() {
            format!("{file_ref}={short}")
        } else {
            short.to_string()
        };
        let out = format!(
            "{head} [auto-delta] ∆{}L\n{diff}\n{savings}",
            disk_content.lines().count()
        );
        return (out, diff_tokens);
    }

    format_full_output(
        file_ref,
        short,
        ext,
        &disk_content,
        store_result.original_tokens,
        store_result.line_count,
        task,
    )
}

fn handle_diff(cache: &mut SessionCache, path: &str, file_ref: &str) -> (String, usize) {
    let _mode_guard = crate::core::savings_footer::ModeGuard::new("diff");
    let short = protocol::shorten_path(path);
    let old_content = cache
        .get(path)
        .and_then(crate::core::cache::CacheEntry::content);

    let new_content = match read_file_lossy(path) {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("ERROR: {e}");
            let tokens = count_tokens(&msg);
            return (msg, tokens);
        }
    };

    let original_tokens = count_tokens(&new_content);

    let diff_output = if let Some(old) = &old_content {
        compressor::diff_content(old, &new_content)
    } else {
        // No previous version cached — store content for future diffs but
        // return a short guidance message instead of dumping the full file.
        cache.store(path, &new_content);
        let msg = format!(
            "{file_ref}={short} [no cached version for diff — use mode=full first, then diff on re-read]"
        );
        let sent = count_tokens(&msg);
        return (msg, sent);
    };

    cache.store(path, &new_content);

    let sent = count_tokens(&diff_output);
    let savings = protocol::format_savings(original_tokens, sent);
    let head = if crate::core::protocol::meta_visible() && !file_ref.is_empty() {
        format!("{file_ref}={short}")
    } else {
        short
    };
    (format!("{head} [diff]\n{diff_output}\n{savings}"), sent)
}
