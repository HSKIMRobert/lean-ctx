use std::io::Write as _;
use std::path::PathBuf;
use std::time::Duration;

const CRASH_LOOP_WINDOW_SECS: u64 = 30;
const CRASH_LOOP_THRESHOLD: usize = 5;
const CRASH_LOOP_MAX_BACKOFF_SECS: u64 = 60;

pub struct StartupLockGuard {
    path: PathBuf,
}

impl StartupLockGuard {
    pub fn touch(&self) {
        // Update mtime so stale eviction doesn't kill active long-running processes.
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
        {
            let _ = writeln!(f, "{now_ms}");
        }
    }
}

impl Drop for StartupLockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn sanitize_lock_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Best-effort cross-process lock (create_new + stale eviction).
///
/// Returns `None` if the data dir can't be resolved or if the lock can't be acquired
/// within `timeout`.
pub fn try_acquire_lock(
    name: &str,
    timeout: Duration,
    stale_after: Duration,
) -> Option<StartupLockGuard> {
    let dir = crate::core::data_dir::lean_ctx_data_dir().ok()?;
    let _ = std::fs::create_dir_all(&dir);

    let name = sanitize_lock_name(name);
    let path = dir.join(format!(".{name}.lock"));

    let deadline = std::time::Instant::now().checked_add(timeout)?;
    let mut sleep_ms: u64 = 10;

    loop {
        if std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .is_ok()
        {
            return Some(StartupLockGuard { path });
        }

        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                if modified
                    .elapsed()
                    .unwrap_or_default()
                    .saturating_sub(stale_after)
                    > Duration::from_secs(0)
                {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }

        if std::time::Instant::now() >= deadline {
            return None;
        }

        std::thread::sleep(Duration::from_millis(sleep_ms));
        sleep_ms = (sleep_ms.saturating_mul(2)).min(120);
    }
}

/// Detects rapid restart loops (e.g., IDE keeps respawning a crashing MCP server).
/// Records each startup timestamp; if too many happen within the window, sleeps
/// with exponential backoff to break the loop and avoid host degradation.
pub fn crash_loop_backoff(process_name: &str) {
    let Some(dir) = crate::core::data_dir::lean_ctx_data_dir().ok() else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir);
    let ts_path = dir.join(format!(".{}-starts.log", sanitize_lock_name(process_name)));

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let cutoff = now.saturating_sub(CRASH_LOOP_WINDOW_SECS);

    let mut recent: Vec<u64> = std::fs::read_to_string(&ts_path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.trim().parse::<u64>().ok())
        .filter(|&ts| ts >= cutoff)
        .collect();
    recent.push(now);

    if let Ok(mut f) = std::fs::File::create(&ts_path) {
        for ts in &recent {
            let _ = writeln!(f, "{ts}");
        }
    }

    if recent.len() > CRASH_LOOP_THRESHOLD {
        let restarts_over = recent.len() - CRASH_LOOP_THRESHOLD;
        let backoff_secs =
            (2u64.saturating_pow(restarts_over as u32)).min(CRASH_LOOP_MAX_BACKOFF_SECS);
        tracing::warn!(
            "crash-loop detected ({} starts in {CRASH_LOOP_WINDOW_SECS}s), backing off {backoff_secs}s",
            recent.len()
        );
        std::thread::sleep(Duration::from_secs(backoff_secs));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvVarGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &std::path::Path) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.prev.as_deref() {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn lock_acquire_and_release() {
        let _env = crate::core::data_dir::test_env_lock();
        let dir = tempfile::tempdir().unwrap();
        let _guard = EnvVarGuard::set("LEAN_CTX_DATA_DIR", dir.path());

        let g = try_acquire_lock(
            "unit-test",
            Duration::from_millis(200),
            Duration::from_secs(30),
        );
        assert!(g.is_some());

        let lock_path = dir.path().join(".unit-test.lock");
        assert!(lock_path.exists());

        drop(g);
        assert!(!lock_path.exists());
    }

    #[test]
    fn lock_times_out_while_held() {
        let _env = crate::core::data_dir::test_env_lock();
        let dir = tempfile::tempdir().unwrap();
        let _guard = EnvVarGuard::set("LEAN_CTX_DATA_DIR", dir.path());

        let g1 = try_acquire_lock(
            "unit-test-2",
            Duration::from_millis(200),
            Duration::from_secs(30),
        )
        .expect("first lock should acquire");
        let g2 = try_acquire_lock(
            "unit-test-2",
            Duration::from_millis(60),
            Duration::from_secs(30),
        );
        assert!(g2.is_none());

        drop(g1);
        let g3 = try_acquire_lock(
            "unit-test-2",
            Duration::from_millis(200),
            Duration::from_secs(30),
        );
        assert!(g3.is_some());
    }
}
