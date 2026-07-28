//! Process-global telemetry for A2A transport operations.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

static DELIVERIES_ATTEMPTED: AtomicU64 = AtomicU64::new(0);
static DELIVERIES_SUCCEEDED: AtomicU64 = AtomicU64::new(0);
static DELIVERIES_FAILED: AtomicU64 = AtomicU64::new(0);
static DELIVERIES_DLQ: AtomicU64 = AtomicU64::new(0);
static TOTAL_PAYLOAD_BYTES: AtomicU64 = AtomicU64::new(0);
static TOTAL_LATENCY_US: AtomicU64 = AtomicU64::new(0);
static RELAY_HOPS_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Record one bounded A2A transport delivery attempt.
pub fn record_delivery(success: bool, payload_bytes: u64, latency_us: u64) {
    DELIVERIES_ATTEMPTED.fetch_add(1, Ordering::Relaxed);
    TOTAL_PAYLOAD_BYTES.fetch_add(payload_bytes, Ordering::Relaxed);
    TOTAL_LATENCY_US.fetch_add(latency_us, Ordering::Relaxed);
    if success {
        DELIVERIES_SUCCEEDED.fetch_add(1, Ordering::Relaxed);
    } else {
        DELIVERIES_FAILED.fetch_add(1, Ordering::Relaxed);
    }
}

/// Record one delivery routed to the dead-letter queue.
pub fn record_dlq() {
    DELIVERIES_DLQ.fetch_add(1, Ordering::Relaxed);
}

/// Record one relay hop traversed by an A2A delivery.
pub fn record_relay_hop() {
    RELAY_HOPS_TOTAL.fetch_add(1, Ordering::Relaxed);
}

/// Point-in-time snapshot of A2A transport telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportSnapshot {
    pub deliveries_attempted: u64,
    pub deliveries_succeeded: u64,
    pub deliveries_failed: u64,
    pub deliveries_dlq: u64,
    pub total_payload_bytes: u64,
    pub total_latency_us: u64,
    pub relay_hops_total: u64,
    pub success_rate: f64,
    pub avg_latency_us: u64,
    pub avg_payload_bytes: u64,
}

/// Capture current process-global A2A transport telemetry.
pub fn snapshot() -> TransportSnapshot {
    let deliveries_attempted = DELIVERIES_ATTEMPTED.load(Ordering::Relaxed);
    let deliveries_succeeded = DELIVERIES_SUCCEEDED.load(Ordering::Relaxed);
    let deliveries_failed = DELIVERIES_FAILED.load(Ordering::Relaxed);
    let total_payload_bytes = TOTAL_PAYLOAD_BYTES.load(Ordering::Relaxed);
    let total_latency_us = TOTAL_LATENCY_US.load(Ordering::Relaxed);

    TransportSnapshot {
        deliveries_attempted,
        deliveries_succeeded,
        deliveries_failed,
        deliveries_dlq: DELIVERIES_DLQ.load(Ordering::Relaxed),
        total_payload_bytes,
        total_latency_us,
        relay_hops_total: RELAY_HOPS_TOTAL.load(Ordering::Relaxed),
        success_rate: if deliveries_attempted > 0 {
            deliveries_succeeded as f64 / deliveries_attempted as f64
        } else {
            0.0
        },
        avg_latency_us: total_latency_us
            .checked_div(deliveries_attempted)
            .unwrap_or(0),
        avg_payload_bytes: total_payload_bytes
            .checked_div(deliveries_attempted)
            .unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn records_delivery_counters() {
        let _guard = TEST_LOCK.lock().unwrap();
        let before = snapshot();

        record_delivery(true, 128, 40);
        record_delivery(false, 256, 80);

        let after = snapshot();
        assert_eq!(after.deliveries_attempted, before.deliveries_attempted + 2);
        assert_eq!(after.deliveries_succeeded, before.deliveries_succeeded + 1);
        assert_eq!(after.deliveries_failed, before.deliveries_failed + 1);
        assert_eq!(after.total_payload_bytes, before.total_payload_bytes + 384);
        assert_eq!(after.total_latency_us, before.total_latency_us + 120);
    }

    #[test]
    fn snapshot_includes_dlq_relay_and_averages() {
        let _guard = TEST_LOCK.lock().unwrap();
        let before = snapshot();

        record_delivery(true, 300, 90);
        record_dlq();
        record_relay_hop();

        let after = snapshot();
        assert_eq!(after.deliveries_dlq, before.deliveries_dlq + 1);
        assert_eq!(after.relay_hops_total, before.relay_hops_total + 1);
        assert_eq!(
            after.avg_latency_us,
            after.total_latency_us / after.deliveries_attempted
        );
        assert_eq!(
            after.avg_payload_bytes,
            after.total_payload_bytes / after.deliveries_attempted
        );
    }

    #[test]
    fn snapshot_reports_success_rate() {
        let _guard = TEST_LOCK.lock().unwrap();
        record_delivery(true, 1, 1);
        record_delivery(true, 1, 1);
        record_delivery(false, 1, 1);

        let current = snapshot();
        let expected = current.deliveries_succeeded as f64 / current.deliveries_attempted as f64;
        assert!((current.success_rate - expected).abs() < f64::EPSILON);
        assert!((0.0..=1.0).contains(&current.success_rate));
    }
}
