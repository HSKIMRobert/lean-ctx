//! Test-only helpers for mutating the process environment.
//!
//! `std::env::set_var` / `std::env::remove_var` became `unsafe` in Rust 2024
//! because they are not thread-safe: a concurrent environment read from another
//! thread is undefined behaviour. lean-ctx's tests serialize every environment
//! mutation through [`crate::core::data_dir::test_env_lock`], so the precondition
//! holds and the call is sound. Centralising the `unsafe` here documents that
//! invariant exactly once instead of at hundreds of call sites, while keeping
//! `#![warn(clippy::undocumented_unsafe_blocks)]` strict everywhere else.

use std::ffi::OsStr;

/// Sets `key` to `value` in the process environment (test-only).
pub(crate) fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    // SAFETY: tests serialize all environment access through test_env_lock(),
    // so no other thread reads or writes the environment concurrently.
    unsafe { std::env::set_var(key, value) };
}

/// Removes `key` from the process environment (test-only).
pub(crate) fn remove_var<K: AsRef<OsStr>>(key: K) {
    // SAFETY: tests serialize all environment access through test_env_lock(),
    // so no other thread reads or writes the environment concurrently.
    unsafe { std::env::remove_var(key) };
}
