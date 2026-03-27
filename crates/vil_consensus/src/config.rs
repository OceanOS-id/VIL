use crate::strategy::ConsensusStrategy;

/// Configuration for the ConsensusEngine.
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Which combination strategy to use.
    pub strategy: ConsensusStrategy,
    /// Per-provider timeout in milliseconds.
    pub timeout_ms: u64,
    /// Minimum number of successful responses required.
    pub min_responses: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            strategy: ConsensusStrategy::BestOfN,
            timeout_ms: 30_000,
            min_responses: 1,
        }
    }
}
