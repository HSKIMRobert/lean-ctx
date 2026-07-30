//! Unified view of Identity + Presence registries.
//!
//! The identity registry (`agent_registry.rs`) tracks CLI-registered agents
//! with attestation. The presence registry (`agents/registry.rs`) tracks MCP
//! processes with PIDs. This bridge merges both into a single list for
//! display/API.

use std::collections::HashSet;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct UnifiedAgent {
    pub agent_id: String,
    pub source: AgentSource,
    pub role: Option<String>,
    pub status: String,
    pub pid: Option<u32>,
    pub alive: bool,
    pub last_active: Option<String>,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentSource {
    Identity,
    Presence,
    Both,
}

/// Merge identity and presence registries into a unified list.
pub(crate) fn list_unified() -> Vec<UnifiedAgent> {
    let mut result = Vec::new();
    let mut seen_ids = HashSet::new();

    for record in crate::core::agent_registry::list() {
        let alive = record.pid.is_some_and(is_pid_alive);
        let status = match record.status {
            crate::core::agent_registry::AgentStatus::Active => {
                if alive {
                    "active"
                } else {
                    "stale"
                }
            }
            crate::core::agent_registry::AgentStatus::Suspended => "suspended",
            crate::core::agent_registry::AgentStatus::Decommissioned => "decommissioned",
        };
        seen_ids.insert(record.agent_id.clone());
        result.push(UnifiedAgent {
            agent_id: record.agent_id,
            source: AgentSource::Identity,
            role: Some(record.role),
            status: status.to_string(),
            pid: record.pid,
            alive,
            last_active: record.last_seen.or(record.last_heartbeat),
            owner: Some(record.owner),
        });
    }

    if let Some(registry) = super::AgentRegistry::load() {
        for agent in &registry.agents {
            if seen_ids.contains(&agent.agent_id) {
                if let Some(existing) = result
                    .iter_mut()
                    .find(|item| item.agent_id == agent.agent_id)
                {
                    existing.source = AgentSource::Both;
                    existing.pid = Some(agent.pid);
                    existing.alive = is_pid_alive(agent.pid);
                }
                continue;
            }

            let alive = is_pid_alive(agent.pid);
            result.push(UnifiedAgent {
                agent_id: agent.agent_id.clone(),
                source: AgentSource::Presence,
                role: agent.role.clone(),
                status: if alive {
                    agent.status.to_string()
                } else {
                    "stale".to_string()
                },
                pid: Some(agent.pid),
                alive,
                last_active: Some(agent.last_active.to_rfc3339()),
                owner: None,
            });
        }
    }

    result
}

fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: signal 0 only probes whether a process exists; it never
        // delivers a signal or accesses process memory.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::list_unified;

    #[test]
    fn list_unified_returns_empty_on_fresh_install() {
        let _iso = crate::core::data_dir::isolated_data_dir();
        let result = list_unified();
        assert!(result.is_empty());
    }
}
