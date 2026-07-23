use md5::{Digest, Md5};
use std::time::SystemTime;

pub fn file_mtime(path: &str) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

pub fn is_cache_entry_stale(path: &str, cached_mtime: Option<SystemTime>) -> bool {
    let current = file_mtime(path);
    match (cached_mtime, current) {
        // Both unavailable (e.g. WSL DrvFS): can't tell → assume fresh (conservative).
        (None, None) => false,
        // One side missing: metadata changed or appeared/disappeared → stale.
        (Some(_), None) | (None, Some(_)) => true,
        // `!=`, not `>`: a *backward* mtime (git checkout, touch -t, snapshot
        // restore) is just as much a content change as a forward one.
        (Some(cached), Some(current)) => current != cached,
    }
}

/// Files larger than this are not content-hashed for stub verification; the
/// mtime check alone decides. Keeps the stub fast-path O(small-file-read).
const VERIFY_HASH_CAP_BYTES: u64 = 8 * 1024 * 1024;

fn cache_verify_enabled() -> bool {
    std::env::var("LEAN_CTX_CACHE_VERIFY").map_or(true, |v| v != "0")
}

/// Staleness with content verification: like [`is_cache_entry_stale`], but when
/// the mtime claims "unchanged", additionally compares the md5 of the on-disk
/// content against the cached hash.
///
/// mtime alone cannot be trusted for *correctness*: same-second writes are
/// invisible on coarse-granularity filesystems (HFS+ 1s, FAT 2s) and mtimes can
/// be restored by tools. Serving an `[unchanged]` stub for changed content
/// would silently mislead the agent — the worst failure mode a context layer
/// can have. The extra disk read costs microseconds for typical source files;
/// the stub's token savings are unaffected. Opt out: `LEAN_CTX_CACHE_VERIFY=0`.
///
/// Note: entries whose stored content differs from disk by design (e.g. secret
/// redaction) hash differently and therefore never serve stubs — conservative
/// and correct.
pub fn is_cache_entry_stale_verified(
    path: &str,
    cached_mtime: Option<SystemTime>,
    cached_hash: &str,
) -> bool {
    if is_cache_entry_stale(path, cached_mtime) {
        return true;
    }
    if cached_hash.is_empty() || !cache_verify_enabled() {
        return false;
    }
    let Ok(meta) = std::fs::metadata(path) else {
        // Can't stat → never serve a stub on top of it.
        return true;
    };
    if meta.len() > VERIFY_HASH_CAP_BYTES {
        return false;
    }
    match std::fs::read(path) {
        // Hash the same view of the bytes that `store()` hashed (lossy UTF-8).
        Ok(bytes) => compute_md5(&String::from_utf8_lossy(&bytes)) != cached_hash,
        Err(_) => true,
    }
}

pub(super) fn compute_md5(content: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(content.as_bytes());
    crate::core::agent_identity::hex_encode(&hasher.finalize())
}
