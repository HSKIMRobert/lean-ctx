use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

const BASE_READ_TIMEOUT: Duration = Duration::from_secs(10);
const BASE_WRITE_TIMEOUT: Duration = Duration::from_secs(10);

/// Acquire a read lock with an adaptive timeout based on I/O health.
/// Returns `None` on timeout (caller must provide graceful fallback).
/// Records a freeze event for self-healing if the timeout is hit.
pub fn read<T: Send + Sync + 'static>(
    lock: &Arc<RwLock<T>>,
    context: &str,
) -> Option<OwnedRwLockReadGuard<T>> {
    let timeout = crate::core::io_health::adaptive_timeout(BASE_READ_TIMEOUT);
    let lock_clone = lock.clone();
    let result = tokio::task::block_in_place(|| {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(tokio::time::timeout(timeout, lock_clone.read_owned()))
    });
    if let Ok(guard) = result {
        Some(guard)
    } else {
        crate::core::io_health::record_freeze();
        tracing::warn!(
            "bounded_lock: read timeout ({}ms) for {context}; degrading gracefully",
            timeout.as_millis()
        );
        None
    }
}

/// Acquire a write lock with an adaptive timeout based on I/O health.
/// Returns `None` on timeout (caller must provide graceful fallback).
/// Records a freeze event for self-healing if the timeout is hit.
pub fn write<T: Send + Sync + 'static>(
    lock: &Arc<RwLock<T>>,
    context: &str,
) -> Option<OwnedRwLockWriteGuard<T>> {
    let timeout = crate::core::io_health::adaptive_timeout(BASE_WRITE_TIMEOUT);
    let lock_clone = lock.clone();
    let result = tokio::task::block_in_place(|| {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(tokio::time::timeout(timeout, lock_clone.write_owned()))
    });
    if let Ok(guard) = result {
        Some(guard)
    } else {
        crate::core::io_health::record_freeze();
        tracing::warn!(
            "bounded_lock: write timeout ({}ms) for {context}; degrading gracefully",
            timeout.as_millis()
        );
        None
    }
}
