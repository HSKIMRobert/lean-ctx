use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const DEFAULT_MAX_HOPS: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayHop {
    pub agent_id: String,
    pub received_at: DateTime<Utc>,
    pub forwarded_at: Option<DateTime<Utc>>,
    pub processing_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayChain {
    pub hops: Vec<RelayHop>,
    pub max_hops: usize,
}

impl RelayChain {
    pub fn new(max_hops: usize) -> Self {
        Self {
            hops: Vec::new(),
            max_hops,
        }
    }

    pub fn add_hop(&mut self, hop: RelayHop) -> Result<(), RelayError> {
        if self.hops.len() >= self.max_hops {
            return Err(RelayError::MaxHopsExceeded(self.max_hops));
        }
        if self.contains_agent(&hop.agent_id) {
            return Err(RelayError::CycleDetected(hop.agent_id));
        }

        self.hops.push(hop);
        Ok(())
    }

    pub fn depth(&self) -> usize {
        self.hops.len()
    }

    pub fn total_latency_ms(&self) -> u64 {
        self.hops
            .iter()
            .filter_map(|hop| hop.processing_ms)
            .fold(0, u64::saturating_add)
    }

    pub fn contains_agent(&self, agent_id: &str) -> bool {
        self.hops.iter().any(|hop| hop.agent_id == agent_id)
    }

    pub fn origin(&self) -> Option<&str> {
        self.hops.first().map(|hop| hop.agent_id.as_str())
    }
}

impl Default for RelayChain {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_HOPS)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, thiserror::Error)]
pub enum RelayError {
    #[error("relay chain exceeded maximum of {0} hops")]
    MaxHopsExceeded(usize),
    #[error("relay cycle detected at agent {0}")]
    CycleDetected(String),
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{RelayChain, RelayError, RelayHop};

    fn hop(agent_id: &str, processing_ms: Option<u64>) -> RelayHop {
        RelayHop {
            agent_id: agent_id.to_owned(),
            received_at: Utc.timestamp_millis_opt(1_700_000_000_000).unwrap(),
            forwarded_at: None,
            processing_ms,
        }
    }

    #[test]
    fn adds_hop_and_reports_chain_metadata() {
        let mut chain = RelayChain::new(3);

        chain.add_hop(hop("origin-agent", Some(4))).unwrap();

        assert_eq!(chain.depth(), 1);
        assert!(chain.contains_agent("origin-agent"));
        assert_eq!(chain.origin(), Some("origin-agent"));
    }

    #[test]
    fn rejects_cycle() {
        let mut chain = RelayChain::new(3);
        chain.add_hop(hop("relay-agent", None)).unwrap();

        let error = chain.add_hop(hop("relay-agent", Some(1))).unwrap_err();

        assert_eq!(error, RelayError::CycleDetected("relay-agent".to_owned()));
        assert_eq!(chain.depth(), 1);
    }

    #[test]
    fn rejects_hop_beyond_maximum() {
        let mut chain = RelayChain::new(1);
        chain.add_hop(hop("origin-agent", None)).unwrap();

        let error = chain.add_hop(hop("next-agent", None)).unwrap_err();

        assert_eq!(error, RelayError::MaxHopsExceeded(1));
        assert_eq!(chain.depth(), 1);
    }

    #[test]
    fn sums_available_processing_latency() {
        let mut chain = RelayChain::new(3);
        chain.add_hop(hop("origin-agent", Some(12))).unwrap();
        chain.add_hop(hop("relay-agent", None)).unwrap();
        chain.add_hop(hop("recipient-agent", Some(8))).unwrap();

        assert_eq!(chain.total_latency_ms(), 20);
    }

    #[test]
    fn default_chain_allows_five_hops() {
        let mut chain = RelayChain::default();
        for index in 0..5 {
            chain
                .add_hop(hop(&format!("agent-{index}"), Some(u64::MAX)))
                .unwrap();
        }

        assert_eq!(chain.depth(), 5);
        assert_eq!(chain.total_latency_ms(), u64::MAX);
        assert_eq!(
            chain.add_hop(hop("sixth-agent", None)),
            Err(RelayError::MaxHopsExceeded(5))
        );
    }
}
