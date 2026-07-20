use crate::core::cache::SessionCache;
use crate::core::intent_engine::{classify, route_intent};
use crate::core::intent_protocol::{IntentRecord, IntentSubject};
use crate::core::ocla::types::{IntentRequest, OclaRequestContext};
use crate::tools::CrpMode;

pub fn handle(
    _cache: &mut SessionCache,
    query: &str,
    project_root: &str,
    _crp_mode: CrpMode,
    format: Option<&str>,
) -> String {
    if query.trim().is_empty() {
        return "ERROR: ctx_intent requires query".to_string();
    }

    let intent = crate::core::intent_protocol::intent_from_query(query, Some(project_root));
    let classification = classify(query);
    let route = route_intent(query, &classification);
    let mut route_v1 = crate::core::intent_router::route_v1(query);
    if let Some(mode) = classify_read_mode(query, project_root, &route_v1) {
        route_v1.decision.effective_read_mode = mode;
    }

    if matches!(format.map(|s| s.trim().to_lowercase()), Some(ref f) if f == "json") {
        return serde_json::to_string_pretty(&route_v1).unwrap_or_else(|e| format!("ERROR: {e}"));
    }

    format_ack(&intent, &route, &route_v1)
}

fn classify_read_mode(
    query: &str,
    project_root: &str,
    route: &crate::core::intent_router::IntentRouteV1,
) -> Option<String> {
    let mut candidate_intents = vec![route.decision.effective_read_mode.clone()];
    if route.decision.recommended_read_mode != candidate_intents[0] {
        candidate_intents.push(route.decision.recommended_read_mode.clone());
    }

    let request = IntentRequest {
        context: OclaRequestContext {
            request_id: format!("ctx-intent:{}", crate::core::hasher::hash_str(query)),
            session_id: format!("project:{}", crate::core::hasher::hash_str(project_root)),
            agent_id: "ctx_intent".to_string(),
            content_ref: format!("query:{}", crate::core::hasher::hash_str(query)),
            tenant_id: None,
        },
        candidate_intents,
    };

    crate::core::ocla::OclaRegistry::global()
        .intent_classifier
        .classify_intent(request)
        .ok()
        .map(|decision| decision.intent)
}

fn format_ack(
    intent: &IntentRecord,
    route: &crate::core::intent_engine::IntentRoute,
    route_v1: &crate::core::intent_router::IntentRouteV1,
) -> String {
    format!(
        "INTENT_OK id={} type={} source={} conf={:.0}% subj={} | route_v1: task={} dimension={} model_tier={}→{} read={} reason={}",
        intent.id,
        intent.intent_type.as_str(),
        intent.source.as_str(),
        (intent.confidence.clamp(0.0, 1.0) * 100.0).round(),
        subject_short(&intent.subject),
        route_v1.inputs.task_type.as_str(),
        route.dimension.as_str(),
        route.model_tier.as_str(),
        route_v1.decision.effective_model_tier.as_str(),
        route_v1.decision.effective_read_mode,
        route.reasoning,
    )
}

fn subject_short(s: &IntentSubject) -> String {
    match s {
        IntentSubject::Project { root } => format!("project({})", root.as_deref().unwrap_or(".")),
        IntentSubject::Command { command } => format!("cmd({})", truncate(command, 80)),
        IntentSubject::Workflow { action } => format!("workflow({})", truncate(action, 60)),
        IntentSubject::KnowledgeFact { category, key, .. } => format!("fact({category}/{key})"),
        IntentSubject::KnowledgeQuery { category, query } => format!(
            "recall({}/{})",
            category.as_deref().unwrap_or("-"),
            query.as_deref().unwrap_or("-")
        ),
        IntentSubject::Tool { name } => format!("tool({name})"),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i + 1 >= max {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}
