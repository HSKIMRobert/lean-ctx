//! `/api/telemetry` — anonymous telemetry heartbeat status and history.

use serde_json::json;

pub(super) fn handle(
    path: &str,
    _query_str: &str,
    _method: &str,
    _body: &str,
) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/telemetry" => Some(telemetry_status()),
        _ => None,
    }
}

fn telemetry_status() -> (&'static str, &'static str, String) {
    let cfg = crate::core::config::Config::load();

    let installation_id = crate::core::installation_id::get_or_create().unwrap_or_default();
    let masked_id = crate::core::installation_id::masked(&installation_id);

    let history = crate::core::telemetry_ledger::read_all();
    let total_sent = history.len();

    let history_entries: Vec<serde_json::Value> = history
        .iter()
        .rev()
        .take(100)
        .map(|r| {
            json!({
                "timestamp": r.timestamp,
                "version": r.version,
                "os": r.os,
                "arch": r.arch,
            })
        })
        .collect();

    let version_dist = compute_distribution(&history, |r| r.version.clone());
    let os_dist = compute_distribution(&history, |r| r.os.clone());
    let arch_dist = compute_distribution(&history, |r| r.arch.clone());

    let daily_counts = compute_daily_counts(&history);

    let payload = json!({
        "enabled": cfg.telemetry.enabled,
        "installation_id": masked_id,
        "last_heartbeat": cfg.telemetry.last_heartbeat,
        "total_sent": total_sent,
        "current_payload": {
            "installation_id": masked_id,
            "version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        },
        "history": history_entries,
        "distributions": {
            "version": version_dist,
            "os": os_dist,
            "arch": arch_dist,
        },
        "daily_counts": daily_counts,
    });

    (
        "200 OK",
        "application/json",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}

fn compute_distribution(
    history: &[crate::core::telemetry_ledger::HeartbeatRecord],
    key_fn: impl Fn(&crate::core::telemetry_ledger::HeartbeatRecord) -> String,
) -> Vec<serde_json::Value> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for record in history {
        *counts.entry(key_fn(record)).or_default() += 1;
    }
    let mut entries: Vec<_> = counts.into_iter().collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.1));
    entries
        .into_iter()
        .map(|(label, count)| json!({"label": label, "count": count}))
        .collect()
}

fn compute_daily_counts(
    history: &[crate::core::telemetry_ledger::HeartbeatRecord],
) -> Vec<serde_json::Value> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for record in history {
        let date = record.timestamp.get(..10).unwrap_or(&record.timestamp);
        *counts.entry(date.to_string()).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(date, count)| json!({"date": date, "count": count}))
        .collect()
}
