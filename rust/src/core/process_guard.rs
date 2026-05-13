//! Global concurrency limiter for lean-ctx processes.
//!
//! Prevents runaway CPU usage by limiting the number of concurrent lean-ctx
//! processes to `MAX_CONCURRENT`. Each process acquires a numbered lock slot
//! under `~/.lean-ctx/locks/`. If all slots are taken, the caller gets `None`.

use std::fs::File;
use std::path::PathBuf;

const MAX_CONCURRENT: usize = 4;

pub struct ProcessGuard {
    _file: File,
    path: PathBuf,
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn lock_dir() -> Option<PathBuf> {
    let dir = crate::core::data_dir::lean_ctx_data_dir()
        .ok()?
        .join("locks");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}

/// Try to acquire one of N concurrent process slots.
/// Returns `None` if all slots are occupied (= too many lean-ctx already running).
pub fn acquire() -> Option<ProcessGuard> {
    let dir = lock_dir()?;

    for slot in 0..MAX_CONCURRENT {
        let path = dir.join(format!("slot-{slot}.lock"));

        let Ok(file) = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
        else {
            continue;
        };

        if try_flock(&file) {
            use std::io::Write;
            let mut f = file;
            let _ = f.write_all(format!("{}", std::process::id()).as_bytes());
            return Some(ProcessGuard { _file: f, path });
        }
    }

    None
}

/// Checks how many slots are currently held (best-effort).
pub fn active_count() -> usize {
    let Some(dir) = lock_dir() else { return 0 };
    let mut count = 0;
    for slot in 0..MAX_CONCURRENT {
        let path = dir.join(format!("slot-{slot}.lock"));
        if let Ok(f) = std::fs::OpenOptions::new().read(true).open(&path) {
            if !try_flock(&f) {
                count += 1;
            }
        }
    }
    count
}

#[cfg(unix)]
fn try_flock(file: &File) -> bool {
    use std::os::unix::io::AsRawFd;
    let fd = file.as_raw_fd();
    let rc = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    rc == 0
}

#[cfg(not(unix))]
fn try_flock(_file: &File) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_and_release() {
        let guard = acquire();
        assert!(guard.is_some(), "should acquire first slot");
        drop(guard);
    }

    #[cfg(unix)]
    #[test]
    fn active_count_reflects_held_slots() {
        let g1 = acquire();
        assert!(g1.is_some());
        let count = active_count();
        assert!(count >= 1, "at least one slot held, got {count}");
        drop(g1);
    }
}
