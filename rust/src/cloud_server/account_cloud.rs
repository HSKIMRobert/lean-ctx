//! `GET /api/account/cloud` — the logged-in user's Personal Cloud dashboard.
//!
//! The website's Personal Cloud page reads this single endpoint to decide
//! between the upsell CTA and the live dashboard. It returns:
//! - `cloud_sync` — the entitlement gate (Pro/Team/Enterprise, or an open
//!   deployment) that flips the page from upsell to dashboard,
//! - `plan` — the resolved plan, for the badge + copy,
//! - `buckets` — a privacy-preserving footprint of what this account has synced
//!   (per-bucket row counts + last-synced timestamps),
//! - `buddy` — the synced buddy state, when present,
//! - `last_synced_at` — the most recent sync across every bucket.
//!
//! The synced *content* never leaves the account; only its shape is surfaced.
//! A failing bucket query degrades to an empty bucket rather than failing the
//! whole dashboard, and accounts without the entitlement get the upsell payload.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};

use super::auth::{auth_user, AppState};
use super::billing_edge::{cloud_sync_allowed, resolve_plan};
use super::helpers::internal_error;

/// Synced buckets surfaced on the dashboard: `(json key, table, timestamp col)`.
/// All three are compile-time constants — never user input — so interpolating
/// the table/column into the aggregate query is injection-safe.
const BUCKETS: [(&str, &str, &str); 6] = [
    ("knowledge", "knowledge_entries", "updated_at"),
    ("commands", "command_stats", "updated_at"),
    ("cep", "cep_scores", "recorded_at"),
    ("gain", "gain_scores", "recorded_at"),
    ("gotchas", "gotchas", "updated_at"),
    ("feedback", "feedback_thresholds", "updated_at"),
];

pub(super) async fn get_account_cloud(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let plan = resolve_plan(&state.cfg, user_id).await;

    // No `cloud_sync` entitlement ⇒ the page renders the gated upsell. Still a
    // 200 with the plan so the CTA can tailor its copy.
    if !cloud_sync_allowed(&state.cfg, plan) {
        return Ok(Json(json!({ "cloud_sync": false, "plan": plan.as_str() })));
    }

    let client = state.pool.get().await.map_err(internal_error)?;

    let mut buckets = Map::new();
    let mut latest: Option<DateTime<Utc>> = None;
    for (key, table, ts_col) in BUCKETS {
        let sql = format!("SELECT COUNT(*)::bigint, MAX({ts_col}) FROM {table} WHERE user_id = $1");
        // A missing/locked bucket must never break the dashboard.
        let (count, last): (i64, Option<DateTime<Utc>>) =
            match client.query_one(&sql, &[&user_id]).await {
                Ok(row) => (row.get(0), row.get(1)),
                Err(_) => (0, None),
            };
        merge_latest(&mut latest, last);
        buckets.insert(
            key.to_string(),
            json!({ "count": count, "last_synced_at": last.map(|t| t.to_rfc3339()) }),
        );
    }

    let buddy = match client
        .query_opt(
            "SELECT name, species, level, xp, mood, streak, rarity, updated_at \
             FROM buddy_state WHERE user_id = $1",
            &[&user_id],
        )
        .await
    {
        Ok(Some(r)) => {
            let last: Option<DateTime<Utc>> = r.get(7);
            merge_latest(&mut latest, last);
            json!({
                "present": true,
                "name": r.get::<_, Option<String>>(0),
                "species": r.get::<_, Option<String>>(1),
                "level": r.get::<_, i32>(2),
                "xp": r.get::<_, i64>(3),
                "mood": r.get::<_, Option<String>>(4),
                "streak": r.get::<_, i32>(5),
                "rarity": r.get::<_, Option<String>>(6),
                "last_synced_at": last.map(|t| t.to_rfc3339()),
            })
        }
        _ => json!({ "present": false }),
    };

    Ok(Json(json!({
        "cloud_sync": true,
        "plan": plan.as_str(),
        "last_synced_at": latest.map(|t| t.to_rfc3339()),
        "buckets": Value::Object(buckets),
        "buddy": buddy,
    })))
}

/// Keep the most recent of the running maximum and a candidate timestamp.
fn merge_latest(latest: &mut Option<DateTime<Utc>>, candidate: Option<DateTime<Utc>>) {
    if let Some(ts) = candidate {
        *latest = Some(latest.map_or(ts, |cur| cur.max(ts)));
    }
}

#[cfg(test)]
mod tests {
    use super::merge_latest;
    use chrono::{TimeZone, Utc};

    #[test]
    fn merge_latest_keeps_the_most_recent_non_null() {
        let mut latest = None;
        let seed = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let older = Utc.with_ymd_and_hms(2025, 6, 1, 0, 0, 0).unwrap();
        let newer = Utc.with_ymd_and_hms(2026, 6, 9, 0, 0, 0).unwrap();

        merge_latest(&mut latest, Some(seed)); // first value seeds the max
        assert_eq!(latest, Some(seed));
        merge_latest(&mut latest, None); // None never lowers the max
        assert_eq!(latest, Some(seed));
        merge_latest(&mut latest, Some(older)); // an older bucket does not win
        assert_eq!(latest, Some(seed));
        merge_latest(&mut latest, Some(newer)); // a newer bucket advances it
        assert_eq!(latest, Some(newer));
    }
}
