//! Provider cache diagnostics integration (#1311).
//!
//! Monitor provider-side prompt cache hit rates and alert on regressions.
//! Anthropic offers 90% cost reduction on cache hits; a drop in hit rate
//! is a cost incident. This module provides the monitoring infrastructure.

use std::sync::atomic::{AtomicU64, Ordering};

static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);
static CACHE_READ_TOKENS: AtomicU64 = AtomicU64::new(0);

/// Record a cache hit from provider response headers.
pub(crate) fn record_hit(read_tokens: u64) {
    CACHE_HITS.fetch_add(1, Ordering::Relaxed);
    CACHE_READ_TOKENS.fetch_add(read_tokens, Ordering::Relaxed);
}

/// Record a cache miss.
pub(crate) fn record_miss() {
    CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
}

/// Current cache hit rate (0.0–1.0), or None if no samples yet.
pub(crate) fn hit_rate() -> Option<f64> {
    let hits = CACHE_HITS.load(Ordering::Relaxed);
    let misses = CACHE_MISSES.load(Ordering::Relaxed);
    let total = hits + misses;
    if total == 0 {
        return None;
    }
    Some(hits as f64 / total as f64)
}

/// Cache diagnostic snapshot.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CacheDiagnostics {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: Option<f64>,
    pub cache_read_tokens: u64,
    pub estimated_savings_usd: f64,
}

impl CacheDiagnostics {
    /// Take a snapshot of current cache diagnostics.
    pub(crate) fn snapshot() -> Self {
        let hits = CACHE_HITS.load(Ordering::Relaxed);
        let misses = CACHE_MISSES.load(Ordering::Relaxed);
        let read_tokens = CACHE_READ_TOKENS.load(Ordering::Relaxed);
        let total = hits + misses;
        let rate = if total > 0 {
            Some(hits as f64 / total as f64)
        } else {
            None
        };

        // Anthropic: cache read = $0.30/Mtok, fresh input = $3.00/Mtok for Sonnet
        let savings_per_token = (3.00 - 0.30) / 1_000_000.0;
        let estimated_savings_usd = read_tokens as f64 * savings_per_token;

        Self {
            hits,
            misses,
            hit_rate: rate,
            cache_read_tokens: read_tokens,
            estimated_savings_usd,
        }
    }

    /// Check if hit rate is below the alert threshold.
    pub(crate) fn needs_alert(&self, threshold: f64) -> bool {
        self.hit_rate.is_some_and(|rate| {
            let total = self.hits + self.misses;
            total >= 10 && rate < threshold
        })
    }
}

/// Alert severity for cache regressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CacheAlertSeverity {
    Warning,
    Critical,
}

/// Generate a cache alert if hit rate is below thresholds.
pub(crate) fn check_alert() -> Option<(CacheAlertSeverity, String)> {
    let diag = CacheDiagnostics::snapshot();
    if let Some(rate) = diag.hit_rate {
        let total = diag.hits + diag.misses;
        if total < 10 {
            return None;
        }
        if rate < 0.50 {
            return Some((
                CacheAlertSeverity::Critical,
                format!(
                    "Provider cache hit rate critically low: {:.0}% ({} hits / {} total)",
                    rate * 100.0,
                    diag.hits,
                    total
                ),
            ));
        }
        if rate < 0.80 {
            return Some((
                CacheAlertSeverity::Warning,
                format!(
                    "Provider cache hit rate below target: {:.0}% ({} hits / {} total)",
                    rate * 100.0,
                    diag.hits,
                    total
                ),
            ));
        }
    }
    None
}

/// Reset counters (for testing).
pub(crate) fn reset() {
    CACHE_HITS.store(0, Ordering::Relaxed);
    CACHE_MISSES.store(0, Ordering::Relaxed);
    CACHE_READ_TOKENS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        reset();
    }

    #[test]
    fn no_data_returns_none() {
        setup();
        assert_eq!(hit_rate(), None);
    }

    #[test]
    fn hit_rate_calculation() {
        setup();
        for _ in 0..8 {
            record_hit(1000);
        }
        for _ in 0..2 {
            record_miss();
        }
        let rate = hit_rate().unwrap();
        assert!((rate - 0.8).abs() < 0.01);
    }

    #[test]
    fn diagnostics_snapshot() {
        setup();
        record_hit(100_000);
        record_miss();
        let diag = CacheDiagnostics::snapshot();
        assert_eq!(diag.hits, 1);
        assert_eq!(diag.misses, 1);
        assert_eq!(diag.cache_read_tokens, 100_000);
        assert!(diag.estimated_savings_usd > 0.0);
    }

    #[test]
    fn alert_below_threshold() {
        setup();
        for _ in 0..3 {
            record_hit(1000);
        }
        for _ in 0..7 {
            record_miss();
        }
        let diag = CacheDiagnostics::snapshot();
        assert!(diag.needs_alert(0.80));
    }

    #[test]
    fn no_alert_when_above_threshold() {
        setup();
        for _ in 0..9 {
            record_hit(1000);
        }
        record_miss();
        let diag = CacheDiagnostics::snapshot();
        assert!(!diag.needs_alert(0.80));
    }

    #[test]
    fn critical_alert_below_50() {
        setup();
        for _ in 0..3 {
            record_hit(1000);
        }
        for _ in 0..7 {
            record_miss();
        }
        let alert = check_alert();
        assert!(alert.is_some());
        let (severity, _) = alert.unwrap();
        assert_eq!(severity, CacheAlertSeverity::Critical);
    }
}
