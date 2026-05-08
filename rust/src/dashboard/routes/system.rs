pub fn handle(
    path: &str,
    _query_str: &str,
    _method: &str,
    _body: &str,
) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/profile" => {
            let active_name = crate::core::profiles::active_profile_name();
            let profile = crate::core::profiles::active_profile();
            let all = crate::core::profiles::list_profiles();
            let active_info = all.iter().find(|p| p.name == active_name);
            let available: Vec<serde_json::Value> = all
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "name": p.name,
                        "description": p.description,
                        "source": p.source.to_string(),
                    })
                })
                .collect();
            let payload = serde_json::json!({
                "active_name": active_name,
                "active_source": active_info.map(|i| i.source.to_string()),
                "active_description": active_info.map(|i| i.description.clone()),
                "profile": profile,
                "available": available,
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/buddy" => {
            let buddy = crate::core::buddy::BuddyState::compute();
            let json = serde_json::to_string(&buddy).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/version" => {
            let json = crate::core::version_check::version_info_json();
            Some(("200 OK", "application/json", json))
        }
        "/metrics" => {
            let prom = crate::core::telemetry::global_metrics().to_prometheus();
            Some(("200 OK", "text/plain; version=0.0.4; charset=utf-8", prom))
        }
        "/api/anomaly" => {
            let s = crate::core::anomaly::summary();
            let json = serde_json::to_string(&s).unwrap_or_else(|_| "[]".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/verification" => {
            let snap = crate::core::output_verification::stats_snapshot();
            let json = serde_json::to_string(&snap).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/slos" => {
            let snap = crate::core::slo::evaluate_quiet();
            let history = crate::core::slo::violation_history(100);
            let payload = serde_json::json!({
                "snapshot": snap,
                "history": history,
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/feedback" => {
            let store = crate::core::feedback::FeedbackStore::load();
            let json = serde_json::to_string(&store).unwrap_or_else(|_| {
                "{\"error\":\"failed to serialize feedback store\"}".to_string()
            });
            Some(("200 OK", "application/json", json))
        }
        _ => None,
    }
}
