use crate::core::consolidation;
use crate::core::providers::config::GitLabConfig;
use crate::core::providers::provider_trait::ProviderParams;
use crate::core::providers::registry::global_registry;
use crate::core::providers::{gitlab, ProviderResult};
use crate::server::tool_trait::ToolContext;

pub fn handle(args: &serde_json::Map<String, serde_json::Value>, ctx: &ToolContext) -> String {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");

    match action {
        // -- Discovery --
        "discover" => handle_discover(),

        // -- Registry-based routing (provider_id + resource) --
        "query" => handle_registry_query(args, ctx),

        // -- Legacy GitLab actions (backward-compatible) --
        "gitlab_issues" => handle_gitlab_issues(args),
        "gitlab_issue" => handle_gitlab_issue(args),
        "gitlab_mrs" => handle_gitlab_mrs(args),
        "gitlab_pipelines" => handle_gitlab_pipelines(args),

        _ => {
            let available =
                "discover, query, gitlab_issues, gitlab_issue, gitlab_mrs, gitlab_pipelines";
            format!("Unknown action: {action}. Available: {available}")
        }
    }
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

fn handle_discover() -> String {
    crate::core::providers::init::init_builtin_providers();
    let infos = global_registry().discover();
    if infos.is_empty() {
        return "No providers registered. Set GITHUB_TOKEN or GITLAB_TOKEN.".to_string();
    }

    let mut out = format!("Registered providers ({}):\n", infos.len());
    for info in &infos {
        let status = if info.available {
            "ready"
        } else {
            "unavailable"
        };
        out.push_str(&format!(
            "  {} ({}) [{}] actions: {}\n",
            info.id,
            info.display_name,
            status,
            info.actions.join(", "),
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Registry-based query (new unified interface)
// ---------------------------------------------------------------------------

fn handle_registry_query(
    args: &serde_json::Map<String, serde_json::Value>,
    ctx: &ToolContext,
) -> String {
    crate::core::providers::init::init_builtin_providers();

    let Some(provider_id) = args.get("provider").and_then(|v| v.as_str()) else {
        return "Error: 'provider' is required for action=query".to_string();
    };
    let Some(resource) = args.get("resource").and_then(|v| v.as_str()) else {
        return "Error: 'resource' is required for action=query".to_string();
    };

    let params = ProviderParams {
        project: args
            .get("project")
            .and_then(|v| v.as_str())
            .map(String::from),
        state: args.get("state").and_then(|v| v.as_str()).map(String::from),
        limit: args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .map(|n| n as usize),
        query: args.get("query").and_then(|v| v.as_str()).map(String::from),
        id: args.get("id").and_then(|v| v.as_str()).map(String::from),
    };

    let mode = args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("compact");

    match mode {
        "chunks" => handle_registry_chunks(provider_id, resource, &params, ctx),
        _ => handle_registry_compact(provider_id, resource, &params, ctx),
    }
}

fn handle_registry_compact(
    provider_id: &str,
    resource: &str,
    params: &ProviderParams,
    ctx: &ToolContext,
) -> String {
    match global_registry().execute_as_chunks(provider_id, resource, params) {
        Ok(chunks) => {
            consolidate_to_session(&chunks, ctx);
            let result = global_registry().execute(provider_id, resource, params);
            match result {
                Ok(r) => format_result(&r),
                Err(_) => format_chunks_compact(&chunks, provider_id, resource),
            }
        }
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_registry_chunks(
    provider_id: &str,
    resource: &str,
    params: &ProviderParams,
    ctx: &ToolContext,
) -> String {
    match global_registry().execute_as_chunks(provider_id, resource, params) {
        Ok(chunks) => {
            consolidate_to_session(&chunks, ctx);
            let mut out = format!(
                "{} content chunks from {provider_id}/{resource}:\n",
                chunks.len()
            );
            for c in &chunks {
                let refs = if c.references.is_empty() {
                    String::new()
                } else {
                    format!(" refs:[{}]", c.references.join(","))
                };
                out.push_str(&format!(
                    "  {} {:?} ({}tok){}\n",
                    c.file_path, c.kind, c.token_count, refs
                ));
            }
            out
        }
        Err(e) => format!("Error: {e}"),
    }
}

/// Consolidate provider chunks into the session cache and graph edges.
/// This is the "sleep replay" wiring: raw provider data flows into
/// the session cache for fast re-reads and cross-source edge creation.
fn consolidate_to_session(chunks: &[crate::core::content_chunk::ContentChunk], ctx: &ToolContext) {
    if chunks.is_empty() {
        return;
    }

    let artifacts = consolidation::consolidate(chunks);
    if artifacts.is_empty() {
        return;
    }

    if let Some(cache_lock) = ctx.cache.as_ref() {
        if let Ok(mut cache) = cache_lock.try_write() {
            for entry in &artifacts.cache_entries {
                cache.store(&entry.uri, &entry.content);
            }
        }
    }

    tracing::debug!(
        "[ctx_provider] consolidated {} chunks → {} edges, {} facts, {} cached",
        artifacts
            .bm25_chunks
            .iter()
            .filter(|c| c.is_external())
            .count(),
        artifacts.edges.len(),
        artifacts.facts.len(),
        artifacts.cache_entries.len(),
    );
}

fn format_chunks_compact(
    chunks: &[crate::core::content_chunk::ContentChunk],
    provider_id: &str,
    resource: &str,
) -> String {
    let mut out = format!("{} results from {provider_id}/{resource}:\n", chunks.len());
    for c in chunks {
        out.push_str(&format!(
            "  #{} {}\n",
            c.file_path.rsplit('/').next().unwrap_or("?"),
            c.symbol_name
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Legacy GitLab handlers (unchanged)
// ---------------------------------------------------------------------------

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
