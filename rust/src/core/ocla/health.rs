//! Aggregated health reporting for the OCLA wire surface.

use std::sync::OnceLock;
use std::time::Instant;

use serde::Serialize;

use crate::core::a2a::dlq::DeadLetterQueue;

use super::registry::OclaRegistry;
use super::types::{OCLA_API_VERSION, OclaCapability, OclaCapabilityKind, OclaCapabilityStatus};
use super::unified_ledger::{FileUnifiedLedger, UnifiedLedger};

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static DLQ: OnceLock<DeadLetterQueue> = OnceLock::new();

pub(crate) fn dead_letter_queue() -> &'static DeadLetterQueue {
    DLQ.get_or_init(DeadLetterQueue::new)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DlqHealthDetails {
    pub total: usize,
    pub oldest_age_secs: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<DlqHealthDetails>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SystemHealth {
    pub overall: HealthStatus,
    pub components: Vec<ComponentHealth>,
    pub uptime_seconds: u64,
    pub version: String,
}

/// Collects health for every OCLA capability and its supporting services.
pub fn check_system_health() -> SystemHealth {
    let started_at = STARTED_AT.get_or_init(Instant::now);
    let registry = OclaRegistry::global();
    let mut components = Vec::with_capacity(OclaCapabilityKind::ALL.len() + 4);

    components.push(poll_capability("observation_hook", || {
        registry.observation_hook.capability()
    }));
    components.push(poll_capability("usage_sink", || {
        registry.usage_sink.capability()
    }));
    components.push(poll_capability("metrics_exporter", || {
        registry.metrics_exporter.capability()
    }));
    components.push(poll_capability("savings_ledger", || {
        registry.savings_ledger.capability()
    }));
    components.push(poll_capability("intent_classifier", || {
        registry.intent_classifier.capability()
    }));
    components.push(poll_capability("outcome_tracker", || {
        registry.outcome_tracker.capability()
    }));
    components.push(poll_capability("compression_provider", || {
        registry.compression_provider.capability()
    }));
    components.push(poll_capability("response_optimizer", || {
        registry.response_optimizer.capability()
    }));
    components.push(poll_capability("model_router", || {
        registry.model_router.capability()
    }));
    components.push(poll_capability("efficiency_analyzer", || {
        registry.efficiency_analyzer.capability()
    }));
    components.push(poll_capability("config_tuner", || {
        registry.config_tuner.capability()
    }));
    components.push(poll_capability("experiment_runner", || {
        registry.experiment_runner.capability()
    }));
    components.push(poll_capability("connector_scheduler", || {
        registry.connector_scheduler.capability()
    }));
    components.push(poll_capability("agent_gateway", || {
        registry.agent_gateway.capability()
    }));

    components.push(check_a2a_bus());
    components.push(check_ledger());
    components.push(check_budget());
    components.push(check_dlq(dead_letter_queue()));

    let overall = aggregate_statuses(&components);
    SystemHealth {
        overall,
        components,
        uptime_seconds: started_at.elapsed().as_secs(),
        version: OCLA_API_VERSION.to_string(),
    }
}

fn poll_capability<F>(name: &str, poll: F) -> ComponentHealth
where
    F: FnOnce() -> OclaCapability,
{
    let started_at = Instant::now();
    let capability = poll();
    let status = match capability.status {
        OclaCapabilityStatus::Available => HealthStatus::Healthy,
        OclaCapabilityStatus::Degraded => {
            HealthStatus::Degraded("capability reports degraded".into())
        }
        OclaCapabilityStatus::Unavailable => {
            HealthStatus::Unhealthy("capability unavailable".into())
        }
    };
    ComponentHealth {
        name: name.to_string(),
        status,
        latency_ms: Some(started_at.elapsed().as_millis() as u64),
        details: None,
    }
}

fn check_a2a_bus() -> ComponentHealth {
    let started_at = Instant::now();
    let status = if crate::core::agents::AgentRegistry::load().is_some() {
        HealthStatus::Healthy
    } else {
        HealthStatus::Degraded("A2A agent registry is unavailable".into())
    };
    ComponentHealth {
        name: "a2a_bus".into(),
        status,
        latency_ms: Some(started_at.elapsed().as_millis() as u64),
        details: None,
    }
}

fn check_ledger() -> ComponentHealth {
    let started_at = Instant::now();
    let status = match FileUnifiedLedger::from_data_dir().and_then(|ledger| ledger.verify_chain()) {
        Ok(true) => HealthStatus::Healthy,
        Ok(false) => HealthStatus::Unhealthy("ledger chain integrity check failed".into()),
        Err(error) => HealthStatus::Unhealthy(format!("ledger is inaccessible: {error}")),
    };
    ComponentHealth {
        name: "ledger".into(),
        status,
        latency_ms: Some(started_at.elapsed().as_millis() as u64),
        details: None,
    }
}

fn check_budget() -> ComponentHealth {
    let started_at = Instant::now();
    let snapshot = crate::core::budget_tracker::BudgetTracker::global().check();
    let status = match snapshot.worst_level() {
        crate::core::budget_tracker::BudgetLevel::Ok => HealthStatus::Healthy,
        crate::core::budget_tracker::BudgetLevel::Warning => {
            HealthStatus::Degraded("runtime budget warning".into())
        }
        crate::core::budget_tracker::BudgetLevel::Exhausted => {
            HealthStatus::Unhealthy("runtime budget exhausted".into())
        }
    };
    ComponentHealth {
        name: "budget".into(),
        status,
        latency_ms: Some(started_at.elapsed().as_millis() as u64),
        details: None,
    }
}

fn check_dlq(queue: &DeadLetterQueue) -> ComponentHealth {
    let started_at = Instant::now();
    let stats = queue.stats();
    let status = if stats.total > 500 {
        HealthStatus::Unhealthy(format!("DLQ contains {} entries", stats.total))
    } else if stats.total > 100 {
        HealthStatus::Degraded(format!("DLQ contains {} entries", stats.total))
    } else {
        HealthStatus::Healthy
    };
    ComponentHealth {
        name: "dlq".into(),
        status,
        latency_ms: Some(started_at.elapsed().as_millis() as u64),
        details: Some(DlqHealthDetails {
            total: stats.total,
            oldest_age_secs: stats.oldest_age_seconds,
        }),
    }
}

fn aggregate_statuses(components: &[ComponentHealth]) -> HealthStatus {
    if let Some(reason) = components
        .iter()
        .find_map(|component| match &component.status {
            HealthStatus::Unhealthy(reason) => Some(reason.clone()),
            _ => None,
        })
    {
        return HealthStatus::Unhealthy(reason);
    }
    if let Some(reason) = components
        .iter()
        .find_map(|component| match &component.status {
            HealthStatus::Degraded(reason) => Some(reason.clone()),
            _ => None,
        })
    {
        return HealthStatus::Degraded(reason);
    }
    HealthStatus::Healthy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(status: HealthStatus) -> ComponentHealth {
        ComponentHealth {
            name: "test".into(),
            status,
            latency_ms: None,
            details: None,
        }
    }

    #[test]
    fn all_healthy_aggregates_to_healthy() {
        let components = vec![
            component(HealthStatus::Healthy),
            component(HealthStatus::Healthy),
        ];
        assert_eq!(aggregate_statuses(&components), HealthStatus::Healthy);
    }

    #[test]
    fn mixed_health_aggregates_to_degraded() {
        let components = vec![
            component(HealthStatus::Healthy),
            component(HealthStatus::Degraded("slow".into())),
        ];
        assert_eq!(
            aggregate_statuses(&components),
            HealthStatus::Degraded("slow".into())
        );
    }

    #[test]
    fn all_unhealthy_aggregates_to_unhealthy() {
        let components = vec![
            component(HealthStatus::Unhealthy("first failed".into())),
            component(HealthStatus::Unhealthy("second failed".into())),
        ];
        assert_eq!(
            aggregate_statuses(&components),
            HealthStatus::Unhealthy("first failed".into())
        );
    }

    #[test]
    fn unhealthy_takes_precedence_over_degraded() {
        let components = vec![
            component(HealthStatus::Degraded("slow".into())),
            component(HealthStatus::Unhealthy("failed".into())),
        ];
        assert_eq!(
            aggregate_statuses(&components),
            HealthStatus::Unhealthy("failed".into())
        );
    }

    #[test]
    fn system_health_reports_all_components() {
        let report = check_system_health();
        assert_eq!(report.components.len(), 18);
        assert_eq!(report.version, OCLA_API_VERSION);
    }

    #[test]
    fn dlq_health_thresholds_and_details_are_reported() {
        let queue = DeadLetterQueue::new();
        let healthy = check_dlq(&queue);
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.details.as_ref().expect("details").total, 0);

        for index in 0..101 {
            queue.enqueue(crate::core::a2a::dlq::DeadLetter {
                id: index.to_string(),
                original_message: "message".into(),
                target_agent: "agent".into(),
                error: "error".into(),
                attempts: 1,
                first_failed_at: "2026-01-01T00:00:00Z".into(),
                last_failed_at: "2026-01-01T00:00:00Z".into(),
            });
        }
        assert!(matches!(
            check_dlq(&queue).status,
            HealthStatus::Degraded(_)
        ));

        for index in 101..501 {
            queue.enqueue(crate::core::a2a::dlq::DeadLetter {
                id: index.to_string(),
                original_message: "message".into(),
                target_agent: "agent".into(),
                error: "error".into(),
                attempts: 1,
                first_failed_at: "2026-01-01T00:00:00Z".into(),
                last_failed_at: "2026-01-01T00:00:00Z".into(),
            });
        }
        assert!(matches!(
            check_dlq(&queue).status,
            HealthStatus::Unhealthy(_)
        ));
    }
}
