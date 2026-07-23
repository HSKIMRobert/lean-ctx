use super::{ReadOutput, count_tokens, format_anchored_output_window};

/// Reads a file as UTF-8 with lossy fallback, enforcing binary detection and max read size limit.
/// Defense-in-depth: verifies that the canonical path stays within the process's project root
/// (if determinable) even though callers SHOULD have already jail-checked the path.
pub fn read_file_lossy(path: &str) -> Result<String, std::io::Error> {
    if crate::core::binary_detect::is_binary_file(path) {
        let msg = crate::core::binary_detect::binary_file_message(path);
        return Err(std::io::Error::other(msg));
    }

    {
        let canonical =
            crate::core::pathutil::safe_canonicalize_bounded(std::path::Path::new(path), 2000);
        if let Ok(cwd) = std::env::current_dir() {
            let root = crate::core::pathutil::safe_canonicalize_bounded(&cwd, 2000);
            if !canonical.starts_with(&root) {
                let allow = crate::core::pathjail::allow_paths_from_env_and_config();
                let data_dir_ok = crate::core::data_dir::lean_ctx_data_dir()
                    .is_ok_and(|d| canonical.starts_with(d));
                let tmp_ok = canonical.starts_with(std::env::temp_dir());
                if !allow.iter().any(|a| canonical.starts_with(a)) && !data_dir_ok && !tmp_ok {
                    tracing::warn!(
                        "defense-in-depth: path may escape project root: {}",
                        canonical.display()
                    );
                }
            }
        }
    }

    let cap = crate::core::limits::max_read_bytes();

    let file = open_with_retry(path)?;
    let meta = file
        .metadata()
        .map_err(|e| std::io::Error::other(format!("cannot stat open file descriptor: {e}")))?;
    if meta.len() > cap as u64 {
        return Err(std::io::Error::other(format!(
            "file too large ({} bytes, limit {} bytes via LCTX_MAX_READ_BYTES). \
             Increase the limit or use a line-range read: mode=\"lines:1-100\"",
            meta.len(),
            cap
        )));
    }

    use std::io::Read;
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    std::io::BufReader::new(file).read_to_end(&mut bytes)?;
    let s = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    };
    Ok(crate::core::io_boundary::strip_utf8_bom(s))
}

/// A streamed line-window read (#811): only the requested span's raw lines,
/// plus the file's true total line count for the header — the rest of the
/// file is never buffered.
pub(super) struct LineWindow {
    /// Raw lines within `[start, end]`, joined with `\n`.
    pub(super) body: String,
    /// True total line count of the file.
    pub(super) total_lines: usize,
    /// Clamped, 1-based inclusive bounds actually served.
    pub(super) start: usize,
    pub(super) end: usize,
}

/// Parses an `anchored:` window payload for the disk-streaming short-circuit
/// below. Only the dash form (`"N-M"`) is fast-pathed — `anchored_lines_mode`
/// (the registered handler) always emits it, using the `999999` EOF sentinel
/// rather than a bare `"N"`. A hand-typed bare payload (meaning "N to EOF")
/// returns `None` and falls through to the normal full-read path instead of
/// guessing a total line count up front.
pub(super) fn parse_disk_anchor_range(payload: &str) -> Option<(usize, usize)> {
    let (s, e) = payload.split_once('-')?;
    let start = s.trim().parse::<usize>().ok()?.max(1);
    let end = e.trim().parse::<usize>().ok()?;
    Some((start, end))
}

/// Streams `path` line-by-line and extracts only `[start, end]` (1-based,
/// inclusive) without ever holding the whole file in memory — the
/// anchored-window counterpart to [`read_file_lossy`] (#811). Every line is
/// still counted (one cheap UTF-8 pass, no per-line allocation outside the
/// requested window) so the caller can report the true total. Returns `None`
/// on anything that isn't a clean streamed text read (I/O error, a binary
/// file, invalid UTF-8 anywhere in the file) so the caller can fall back to
/// the existing, more permissive `read_file_lossy` path — behaviour never
/// regresses, it just doesn't always get the fast path.
pub(super) fn read_line_window(path: &str, start: usize, end: usize) -> Option<LineWindow> {
    if crate::core::binary_detect::is_binary_file(path) {
        return None;
    }
    use std::io::BufRead;
    let file = open_with_retry(path).ok()?;
    let reader = std::io::BufReader::new(file);
    let mut total = 0usize;
    let mut collected = Vec::new();
    for line in reader.lines() {
        let line = line.ok()?;
        total += 1;
        if total >= start && total <= end {
            collected.push(line);
        }
    }
    Some(LineWindow {
        body: collected.join("\n"),
        total_lines: total,
        start: start.min(total.max(1)),
        end: end.min(total),
    })
}

/// #811: attempt the disk-streaming short-circuit for a fresh `anchored:N-M`
/// read. `None` when the request isn't eligible (not a windowed anchored
/// read, or a preread is already in hand — nothing to short-circuit) or the
/// fast path can't run cleanly (binary file, invalid UTF-8, I/O error); the
/// caller falls through to the normal full-read path in that case.
pub(super) fn try_disk_anchored_window(
    path: &str,
    mode: &str,
    fresh: bool,
    preread_is_none: bool,
    file_ref: &str,
    short: &str,
) -> Option<ReadOutput> {
    if !fresh || !preread_is_none {
        return None;
    }
    let range = mode.strip_prefix("anchored:")?;
    let (start, end) = parse_disk_anchor_range(range)?;
    let window = read_line_window(path, start, end)?;
    let (out, _) = format_anchored_output_window(
        file_ref,
        short,
        &window.body,
        window.total_lines,
        Some((window.start, window.end)),
    );
    let out = crate::core::redaction::redact_text_if_enabled(&out);
    let sent = count_tokens(&out);
    Some(ReadOutput {
        content: out,
        resolved_mode: mode.to_string(),
        output_tokens: sent,
        is_cache_hit: false,
    })
}

/// Opens a file, retrying once after a brief pause on NotFound.
/// Works around overlay/FUSE stat-cache races in container runtimes (Docker, Codex).
/// Uses O_NOFOLLOW on Unix for TOCTOU symlink protection.
fn open_with_retry(path: &str) -> Result<std::fs::File, std::io::Error> {
    match open_nofollow(path) {
        Ok(f) => Ok(f),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::thread::sleep(std::time::Duration::from_millis(50));
            open_nofollow(path).map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    std::io::Error::other(format!(
                        "file not found: {path} — verify the path with ctx_tree or ctx_search"
                    ))
                } else {
                    e
                }
            })
        }
        Err(e) => Err(e),
    }
}

#[cfg(unix)]
fn open_nofollow(path: &str) -> Result<std::fs::File, std::io::Error> {
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::Path;

    let p = Path::new(path);
    // Canonicalize the parent directory (resolving symlinks in the directory path)
    // but apply O_NOFOLLOW only to the final file component. This prevents
    // symlink-following attacks on the target file while allowing legitimate
    // directory symlinks (e.g., /tmp → /private/tmp on macOS).
    if let (Some(parent), Some(filename)) = (p.parent(), p.file_name())
        && parent.exists()
    {
        let canonical_parent = crate::core::pathutil::safe_canonicalize_bounded(parent, 2000);
        let canonical_path = canonical_parent.join(filename);
        return std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&canonical_path);
    }

    // Fallback: direct open with O_NOFOLLOW
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(not(unix))]
fn open_nofollow(path: &str) -> Result<std::fs::File, std::io::Error> {
    std::fs::File::open(path)
}
