//! Shared, policy-free atomic-write mechanics.
//!
//! Both the config installer ([`crate::config_io`]) and the edit tools
//! (`crate::tools::edit_io`) need the same durable write dance: a
//! same-directory temp file + `rename`, with an in-place-overwrite fallback when
//! the directory is read-only but the file inode is writable (#459). Only the
//! *mechanism* lives here — one audited implementation. The differing *policy*
//! stays in each caller:
//!
//! * `edit_io` rejects symlinks (`reject_symlink` + `O_NOFOLLOW`) and guards
//!   TOCTOU/read-only-roots before calling in;
//! * `config_io` resolves a user-managed symlink to its real target (within
//!   `$HOME`) and then writes through to it.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn invalid_input(msg: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, msg)
}

/// Durable, crash-atomic write: a temp file in the **same directory** as `path`
/// followed by `rename` over the target. Requires write permission on the parent
/// directory; the read-only-directory fallback is handled by
/// [`write_bytes_with_fallback`].
pub(crate) fn try_atomic_write(
    path: &Path,
    bytes: &[u8],
    permissions: Option<&std::fs::Permissions>,
) -> std::io::Result<()> {
    use std::io::Write;

    let parent = path
        .parent()
        .ok_or_else(|| invalid_input("invalid path (no parent directory)"))?;
    let filename = path
        .file_name()
        .ok_or_else(|| invalid_input("invalid path (no filename)"))?
        .to_string_lossy();

    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp = parent.join(format!(".{filename}.lean-ctx.tmp.{pid}.{nanos}"));

    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        let _ = f.flush();
        let _ = f.sync_all();
    }

    if let Some(perms) = permissions {
        let _ = std::fs::set_permissions(&tmp, perms.clone());
    }

    // #956: on Windows, `rename` fails outright if `path` already exists, so
    // this used to `remove_file(path)` first and rename second — two
    // non-atomic syscalls. A reader landing in that window sees ENOENT for a
    // file that "exists", and a process that dies after the remove but
    // before the rename loses the original file for good while the temp file
    // is left behind orphaned. `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`
    // performs the swap as one atomic operation, matching what `rename(2)`
    // already gives us for free on Unix.
    #[cfg(windows)]
    if let Err(e) = windows_replace(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    #[cfg(not(windows))]
    if let Err(e) = std::fs::rename(&tmp, path) {
        // Don't leave a half-written temp behind before the caller decides
        // whether to fall back.
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    #[cfg(unix)]
    fsync_dir(parent);

    Ok(())
}

/// Atomically replace `path` with `tmp` via `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`
/// — the Windows equivalent of POSIX `rename(2)`'s implicit replace-if-exists.
#[cfg(windows)]
fn windows_replace(tmp: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MoveFileExW};

    fn to_wide(p: &Path) -> Vec<u16> {
        p.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    let tmp_w = to_wide(tmp);
    let path_w = to_wide(path);

    let ok = unsafe { MoveFileExW(tmp_w.as_ptr(), path_w.as_ptr(), MOVEFILE_REPLACE_EXISTING) };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Best-effort `fsync` of a directory's inode so a preceding `rename`/`create`
/// into it survives a crash (Unix only — opening a directory as a `File` for
/// this purpose is not portable to Windows).
#[cfg(unix)]
fn fsync_dir(dir: &Path) {
    if let Ok(f) = std::fs::File::open(dir) {
        let _ = f.sync_all();
    }
}

/// In-place overwrite of an existing file inode (`O_WRONLY|O_TRUNC`, plus
/// `O_NOFOLLOW` on Unix). Works when the parent directory is read-only but the
/// file itself is writable. Not crash-atomic — used only as a fallback when the
/// atomic path is impossible.
pub(crate) fn in_place_overwrite(
    path: &Path,
    bytes: &[u8],
    permissions: Option<&std::fs::Permissions>,
) -> std::io::Result<()> {
    use std::io::Write;

    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // O_NOFOLLOW: a symlink swapped in after the caller's checks must never
        // be followed here (mirrors the read-side O_NOFOLLOW boundary).
        opts.custom_flags(libc::O_NOFOLLOW);
    }

    let mut f = opts.open(path)?;
    f.write_all(bytes)?;
    let _ = f.flush();
    let _ = f.sync_all();

    if let Some(perms) = permissions {
        let _ = std::fs::set_permissions(path, perms.clone());
    }
    Ok(())
}

/// True for errors that mean "this directory won't accept create/rename" even
/// though the target file may be writable: `EROFS` (read-only fs) plus
/// `EACCES`/`EPERM` (directory write denied).
pub(crate) fn is_readonly_dir_error(e: &std::io::Error) -> bool {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        return true;
    }
    #[cfg(unix)]
    {
        matches!(
            e.raw_os_error(),
            Some(libc::EROFS | libc::EACCES | libc::EPERM)
        )
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Atomic write with the read-only-directory in-place fallback (#459). Tries the
/// crash-atomic temp+rename first; if that fails because the *directory* is
/// read-only/permission-denied but an existing file inode is writable, overwrite
/// it in place. `permissions`, when given, is applied to the written file.
pub(crate) fn write_bytes_with_fallback(
    path: &Path,
    bytes: &[u8],
    permissions: Option<&std::fs::Permissions>,
) -> Result<(), String> {
    match try_atomic_write(path, bytes, permissions) {
        Ok(()) => Ok(()),
        Err(e) if is_readonly_dir_error(&e) && path.is_file() => {
            in_place_overwrite(path, bytes, permissions).map_err(|fallback_err| {
                format!(
                    "atomic write failed ({e}); in-place fallback also failed: {fallback_err} ({})",
                    path.display()
                )
            })
        }
        Err(e) => Err(format!("atomic write failed: {e} ({})", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readonly_dir_error_classification() {
        assert!(is_readonly_dir_error(&std::io::Error::from(
            std::io::ErrorKind::PermissionDenied
        )));
        assert!(!is_readonly_dir_error(&std::io::Error::from(
            std::io::ErrorKind::NotFound
        )));
        #[cfg(unix)]
        {
            assert!(is_readonly_dir_error(&std::io::Error::from_raw_os_error(
                libc::EROFS
            )));
            assert!(is_readonly_dir_error(&std::io::Error::from_raw_os_error(
                libc::EACCES
            )));
            assert!(is_readonly_dir_error(&std::io::Error::from_raw_os_error(
                libc::EPERM
            )));
        }
    }

    #[cfg(unix)]
    #[test]
    fn fsync_dir_succeeds_on_a_real_directory() {
        // #954: exercises the open+sync_all mechanics directly — must not
        // panic or otherwise disrupt the caller (best-effort, errors ignored).
        let dir = tempfile::tempdir().unwrap();
        fsync_dir(dir.path());
    }

    #[test]
    fn try_atomic_write_creates_and_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cfg.toml");
        try_atomic_write(&path, b"first", None).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first");
        // No leftover temp files.
        let strays: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".lean-ctx.tmp."))
            .collect();
        assert!(strays.is_empty(), "temp file must not linger");
        try_atomic_write(&path, b"second", None).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
    }

    #[cfg(unix)]
    #[test]
    fn in_place_overwrite_truncates_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.jsonc");
        std::fs::write(&path, b"longer original content").unwrap();
        in_place_overwrite(&path, b"short", None).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"short");
    }

    #[cfg(unix)]
    #[test]
    fn fallback_overwrites_when_parent_dir_is_readonly() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cfg.toml");
        std::fs::write(&path, b"original").unwrap();
        // Read-only parent dir: temp+rename is impossible, but the file inode
        // stays writable, so the in-place fallback must succeed.
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
        let res = write_bytes_with_fallback(&path, b"updated", None);
        let _ = std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700));
        res.expect("read-only-dir fallback must succeed");
        assert_eq!(std::fs::read(&path).unwrap(), b"updated");
    }
}
