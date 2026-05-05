use crate::core::providers::config::GitLabConfig;
use crate::core::providers::{gitlab, ProviderResult};

pub fn handle(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");

    match action {
        "gitlab_issues" => handle_gitlab_issues(args),
        "gitlab_issue" => handle_gitlab_issue(args),
        "gitlab_mrs" => handle_gitlab_mrs(args),
        "gitlab_pipelines" => handle_gitlab_pipelines(args),
        _ => format!(
            "Unknown action: {action}. Available: gitlab_issues, gitlab_issue, gitlab_mrs, gitlab_pipelines"
        ),
    }
}

fn handle_gitlab_issues(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let config = match GitLabConfig::from_env() {
        Ok(c) => c,
        Err(e) => return format!("Error: {e}"),
    };
    let state = args.get("state").and_then(|v| v.as_str());
    let labels = args.get("labels").and_then(|v| v.as_str());
    let limit = args
        .get("limit")
        .and_then(serde_json::Value::as_u64)
        .map(|n| n as usize);

    match gitlab::list_issues(&config, state, labels, limit) {
        Ok(result) => format_result(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_gitlab_issue(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let config = match GitLabConfig::from_env() {
        Ok(c) => c,
        Err(e) => return format!("Error: {e}"),
    };
    let iid = args
        .get("iid")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if iid == 0 {
        return "Error: iid is required for gitlab_issue".to_string();
    }

    match gitlab::show_issue(&config, iid) {
        Ok(result) => format_result(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_gitlab_mrs(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let config = match GitLabConfig::from_env() {
        Ok(c) => c,
        Err(e) => return format!("Error: {e}"),
    };
    let state = args.get("state").and_then(|v| v.as_str());
    let limit = args
        .get("limit")
        .and_then(serde_json::Value::as_u64)
        .map(|n| n as usize);

    match gitlab::list_mrs(&config, state, limit) {
        Ok(result) => format_result(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_gitlab_pipelines(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let config = match GitLabConfig::from_env() {
        Ok(c) => c,
        Err(e) => return format!("Error: {e}"),
    };
    let status = args.get("status").and_then(|v| v.as_str());
    let limit = args
        .get("limit")
        .and_then(serde_json::Value::as_u64)
        .map(|n| n as usize);

    match gitlab::list_pipelines(&config, status, limit) {
        Ok(result) => format_result(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn format_result(result: &ProviderResult) -> String {
    crate::core::redaction::redact_text_if_enabled(&result.format_compact())
}
