//! `/api/roi` — the user-facing **ROI & Plan** monitoring surface.
//!
//! Aggregates read-only, privacy-preserving data for the dashboard's "ROI" view:
//! - the signed savings ROI report (tokens/$ saved, top models/tools, provenance),
//! - the daily savings trend (for the chart),
//! - the effective commercial plan + entitlements (cache-only — applies offline
//!   grace, never hits the network on a dashboard request),
//! - the metered-usage billable flag.
//!
//! It never gates anything and never mutates state. The plan shown here is only
//! for display/hosted-surface hints; the local engine has no entitlement checks
//! (Local-Free Invariant).

use serde_json::json;

pub(super) fn handle(
    path: &str,
    _query_str: &str,
    _method: &str,
    _body: &str,
) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/roi" => Some(roi()),
        "/api/team-roi" => Some(team_roi()),
        _ => None,
    }
}

fn roi() -> (&'static str, &'static str, String) {
    let agent_id = crate::core::agent_identity::current_agent_id();
    let report = crate::core::savings_ledger::roi_report(agent_id);
    let summary = crate::core::savings_ledger::summary();
    let usage = crate::core::billing::metered_usage(agent_id);

    let eff = crate::cloud_client::resolve_effective_plan_cached();
    let entitlements = eff.plan.entitlements();
    let logged_in = crate::cloud_client::is_logged_in();

    let payload = json!({
        "roi": report,
        // [[YYYY-MM-DD, saved_tokens, saved_usd], ...] ascending — drives the trend chart.
        "trend": summary.by_day,
        "plan": {
            "plan": eff.plan.as_str(),
            "source": plan_source_label(eff.source),
            "verified_at": eff.verified_at,
            "grace_days": eff.grace_days,
            "logged_in": logged_in,
            "entitlements": entitlements,
        },
        "usage": {
            "billable": usage.is_billable(),
            "metered_events": usage.metered_events,
            "net_saved_tokens": usage.net_saved_tokens,
            "saved_usd": usage.saved_usd,
        }
    });
    let body = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    ("200 OK", "application/json", body)
}

/// `/api/team-roi` — the opt-in **team** savings roll-up for the dashboard's ROI
/// view. The browser can't reach the team server directly (token + CORS), so the
/// dashboard proxies `GET {team_url}/v1/savings/summary` using the locally
/// configured `team_url` + bearer token. Returns `{configured:false}` when no
/// team server is set, `{configured:true,summary:…}` on success, or
/// `{configured:true,error:…}` when the server is unreachable/denies access.
fn team_roi() -> (&'static str, &'static str, String) {
    let cfg = crate::core::config::Config::load();
    let Some(url) = cfg.team_url.clone().filter(|s| !s.trim().is_empty()) else {
        return ok_json(&json!({ "configured": false }));
    };
    // Token precedence mirrors the CLI: env wins over config.toml.
    let token = std::env::var("LEAN_CTX_TEAM_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or(cfg.team_token);

    let payload = match fetch_team_summary(&url, token) {
        Ok((200, text)) => {
            let summary: serde_json::Value =
                serde_json::from_str(&text).unwrap_or_else(|_| json!({}));
            json!({ "configured": true, "summary": summary })
        }
        Ok((401 | 403, _)) => json!({
            "configured": true,
            "error": "Access denied — the team token needs the 'audit' scope (owner/admin)."
        }),
        Ok((status, _)) => json!({
            "configured": true,
            "error": format!("Team server returned HTTP {status}.")
        }),
        Err(e) => json!({ "configured": true, "error": e }),
    };
    ok_json(&payload)
}

/// Blocking GET of the team savings summary, bounded by a hard timeout so a slow
/// or hung team server can never stall a dashboard worker. Runs the request on a
/// detached thread and waits at most a few seconds for the result.
fn fetch_team_summary(url: &str, token: Option<String>) -> Result<(u16, String), String> {
    let endpoint = format!("{}/v1/savings/summary", url.trim_end_matches('/'));
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut req = ureq::get(&endpoint);
        if let Some(t) = token {
            req = req.header("Authorization", &format!("Bearer {t}"));
        }
        let result = match req.call() {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let text = resp.into_body().read_to_string().unwrap_or_default();
                Ok((status, text))
            }
            Err(e) => Err(format!("Could not reach team server: {e}")),
        };
        let _ = tx.send(result);
    });
    match rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(r) => r,
        Err(_) => Err("Team server timed out.".to_string()),
    }
}

/// Serialize `payload` as a `200 OK` JSON dashboard response.
fn ok_json(payload: &serde_json::Value) -> (&'static str, &'static str, String) {
    let body = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    ("200 OK", "application/json", body)
}

/// Stable wire label for the effective-plan provenance.
fn plan_source_label(source: crate::cloud_client::PlanSource) -> &'static str {
    use crate::cloud_client::PlanSource;
    match source {
        PlanSource::Live => "live",
        PlanSource::Cached => "cached",
        PlanSource::Expired => "expired",
        PlanSource::None => "none",
    }
}
