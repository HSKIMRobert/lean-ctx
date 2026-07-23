pub(super) const DEFAULT_CONTEXT_WINDOW: usize = 128_000;

/// EMA weight for the freshly computed Phi on a re-read (#2). 0.5 keeps equal
/// weight on the new signal and the running history, so salience tracks recency
/// and task changes without overreacting to one read.
pub(super) const PHI_REREAD_ALPHA: f64 = 0.5;

/// Default Global-Workspace ignition threshold (#6) as a Phi z-score: an item
/// must stand more than this many standard deviations above the mean salience to
/// "ignite" and be broadcast (promoted to Pinned) into the global workspace.
pub(super) const GWT_IGNITION_Z: f64 = 1.5;
/// Minimum number of scored entries before ignition can fire — below this the
/// Phi distribution is too small to identify a meaningful outlier, so ignition
/// is suppressed to avoid pinning everything on a cold ledger.
pub(super) const GWT_MIN_ENTRIES: usize = 4;

pub(super) fn ledger_path(agent_id: &str) -> Result<std::path::PathBuf, String> {
    let dir = crate::core::paths::state_dir()?;
    if agent_id == "default" {
        Ok(dir.join("context_ledger.json"))
    } else {
        let ledger_dir = dir.join("ledger");
        let safe_id: String = agent_id
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        Ok(ledger_dir.join(format!("{safe_id}.json")))
    }
}

pub(super) fn atomic_write_json(path: &std::path::Path, data: &str) {
    let _ = crate::config_io::write_atomic(path, data);
}

/// Acquire an advisory file lock for cross-process safety.
/// Returns the lock file handle (lock released on drop).
#[cfg(unix)]
pub(super) fn acquire_ledger_lock(path: &std::path::Path) -> Option<std::fs::File> {
    use std::os::unix::io::AsRawFd;
    let lock_path = path.with_extension("json.lock");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .ok()?;
    let fd = file.as_raw_fd();
    // SAFETY: `fd` is a valid open descriptor owned by `file`, which outlives
    // this call; `flock` dereferences no pointers.
    let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        // Lock held — block up to 2s
        use std::time::{Duration, Instant};
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            std::thread::sleep(Duration::from_millis(50));
            // SAFETY: `fd` is still a valid open descriptor owned by `file`,
            // which outlives this call; `flock` dereferences no pointers.
            let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
            if ret == 0 {
                break;
            }
            if Instant::now() >= deadline {
                return None;
            }
        }
    }
    Some(file)
}

#[cfg(not(unix))]
pub(super) fn acquire_ledger_lock(_path: &std::path::Path) -> Option<std::fs::File> {
    None
}
