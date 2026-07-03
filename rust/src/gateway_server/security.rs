//! Admin-port security hardening (#54): response headers + auth throttling.
//!
//! Split out of `serve.rs` so the wiring stays readable and both pieces are
//! unit-testable in isolation.
//!
//! **Headers** — every admin response carries a strict `Content-Security-Policy`
//! (the console is self-contained: no CDN, no inline scripts), clickjacking and
//! MIME-sniffing guards, and cache rules that keep token-guarded JSON out of
//! shared caches. HSTS is deliberately *not* set here: TLS terminates at the
//! ingress/reverse proxy (see `lean-ctx-deploy` SECURITY.md), and a backend-set
//! HSTS on a plain-HTTP loopback deployment would poison local browsers.
//!
//! **Throttle** — fixed-window failed-auth limiter per client IP. The Bearer
//! token is 256-bit random (brute force is not a practical risk); the limiter
//! exists so scanners/mistyped scripts produce a clean, auditable signal (429 +
//! a `tracing` line per failure) instead of an unbounded 401 stream.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::http::header::{CACHE_CONTROL, HeaderName, HeaderValue};

/// Failed attempts allowed per IP per window before 429.
const MAX_FAILURES_PER_WINDOW: u32 = 10;
/// Window length of the failed-auth limiter.
const WINDOW: Duration = Duration::from_mins(1);
/// Hard cap on tracked IPs (memory guard; oldest windows are pruned lazily).
const MAX_TRACKED_IPS: usize = 10_000;

/// CSP for the embedded console: everything ships from the gateway itself.
/// `img-src data:` covers the inline SVG favicon; no other exception exists.
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
                   img-src 'self' data:; font-src 'self'; connect-src 'self'; \
                   frame-ancestors 'none'; base-uri 'none'; form-action 'self'";

/// Middleware: stamps the security headers on every admin-port response.
pub async fn security_headers(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = req.uri().path().to_string();
    let mut res = next.run(req).await;
    let h = res.headers_mut();

    let set = |h: &mut axum::http::HeaderMap, name: &'static str, value: &'static str| {
        h.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    };
    set(h, "content-security-policy", CSP);
    set(h, "x-content-type-options", "nosniff");
    set(h, "x-frame-options", "DENY");
    set(h, "referrer-policy", "no-referrer");
    set(h, "cross-origin-opener-policy", "same-origin");
    set(h, "cross-origin-resource-policy", "same-origin");

    // Token-guarded payloads must never land in shared caches; immutable
    // static assets may (they change only with the binary). `/me/static/` is
    // the personal view's asset namespace on the proxy port (enterprise#64).
    let cache = if path.starts_with("/api/") || path == "/metrics" {
        "no-store"
    } else if path.starts_with("/static/") || path.starts_with("/me/static/") {
        "public, max-age=3600"
    } else {
        "no-cache"
    };
    h.insert(CACHE_CONTROL, HeaderValue::from_static(cache));
    res
}

/// Fixed-window failed-auth limiter per client IP (#54/#57).
#[derive(Debug, Default)]
pub struct AuthThrottle {
    windows: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl AuthThrottle {
    /// True when `ip` has exhausted its failure budget for the current window
    /// (the caller responds 429 without evaluating credentials).
    pub fn is_blocked(&self, ip: IpAddr) -> bool {
        let mut w = lock(&self.windows);
        match w.get(&ip) {
            Some((start, n)) if start.elapsed() < WINDOW => *n >= MAX_FAILURES_PER_WINDOW,
            Some(_) => {
                w.remove(&ip);
                false
            }
            None => false,
        }
    }

    /// Records a failed attempt; returns the failure count in the window.
    pub fn record_failure(&self, ip: IpAddr) -> u32 {
        let mut w = lock(&self.windows);
        if w.len() >= MAX_TRACKED_IPS && !w.contains_key(&ip) {
            w.retain(|_, (start, _)| start.elapsed() < WINDOW);
            if w.len() >= MAX_TRACKED_IPS {
                // Saturated by active windows — treat the newcomer as blocked
                // rather than growing without bound.
                return MAX_FAILURES_PER_WINDOW;
            }
        }
        let now = Instant::now();
        let entry = w.entry(ip).or_insert((now, 0));
        if entry.0.elapsed() >= WINDOW {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1
    }

    /// Clears the window after a successful authentication.
    pub fn record_success(&self, ip: IpAddr) {
        lock(&self.windows).remove(&ip);
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(last: u8) -> IpAddr {
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, last))
    }

    #[test]
    fn throttle_blocks_after_budget_and_resets_on_success() {
        let t = AuthThrottle::default();
        assert!(!t.is_blocked(ip(1)));
        for _ in 0..MAX_FAILURES_PER_WINDOW {
            t.record_failure(ip(1));
        }
        assert!(t.is_blocked(ip(1)), "budget exhausted → blocked");
        assert!(!t.is_blocked(ip(2)), "per-IP isolation");
        t.record_success(ip(1));
        assert!(!t.is_blocked(ip(1)), "success clears the window");
    }

    #[test]
    fn throttle_counts_failures_within_window() {
        let t = AuthThrottle::default();
        assert_eq!(t.record_failure(ip(3)), 1);
        assert_eq!(t.record_failure(ip(3)), 2);
        assert!(!t.is_blocked(ip(3)), "under budget stays open");
    }

    #[test]
    fn csp_has_no_remote_sources() {
        // The console is fully embedded; any remote origin in the CSP would
        // signal an accidental CDN dependency.
        for directive in CSP.split(';') {
            assert!(
                !directive.contains("http"),
                "CSP must not allow remote origins: {directive}"
            );
        }
        assert!(CSP.contains("frame-ancestors 'none'"));
    }
}
