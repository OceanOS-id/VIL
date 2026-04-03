// =============================================================================
// VIL Server Mesh — Scatter-Gather Pattern
// =============================================================================
//
// Fan-out a request to multiple services, then fan-in (aggregate) responses.
// Uses Tri-Lane channels for zero-copy data passing between stages.
//
// Example:
//   Request → scatter to [svc_a, svc_b, svc_c] → gather all responses → merge
//
// Strategies:
//   - WaitAll: wait for all responses (default)
//   - WaitAny: return as soon as first response arrives
//   - WaitQuorum: return when N out of M respond
//   - Timeout: return whatever arrived within deadline

use bytes::Bytes;
use std::time::Duration;

/// Scatter-gather strategy.
#[derive(Debug, Clone)]
pub enum GatherStrategy {
    /// Wait for all targets to respond
    WaitAll,
    /// Return first response (fastest wins)
    WaitAny,
    /// Wait for N responses out of total
    WaitQuorum { min_responses: usize },
    /// Wait up to deadline, return whatever arrived
    Timeout { deadline: Duration },
}

impl Default for GatherStrategy {
    fn default() -> Self {
        Self::WaitAll
    }
}

/// A single scatter target.
#[derive(Debug, Clone)]
pub struct ScatterTarget {
    pub service: String,
    pub payload: Bytes,
}

/// Result from a single target.
#[derive(Debug, Clone)]
pub struct ScatterResult {
    pub service: String,
    pub success: bool,
    pub response: Option<Bytes>,
    pub error: Option<String>,
    pub latency_ns: u64,
}

/// Scatter-gather request builder.
pub struct ScatterGather {
    targets: Vec<ScatterTarget>,
    strategy: GatherStrategy,
    timeout: Duration,
}

impl ScatterGather {
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            strategy: GatherStrategy::WaitAll,
            timeout: Duration::from_secs(30),
        }
    }

    /// Add a scatter target.
    pub fn target(mut self, service: impl Into<String>, payload: impl Into<Bytes>) -> Self {
        self.targets.push(ScatterTarget {
            service: service.into(),
            payload: payload.into(),
        });
        self
    }

    /// Set the gather strategy.
    pub fn strategy(mut self, strategy: GatherStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the overall timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Execute scatter-gather.
    ///
    /// In production, this sends to services via Tri-Lane mesh.
    /// Returns results from all (or subset based on strategy) targets.
    pub async fn execute(self) -> GatherResult {
        let start = std::time::Instant::now();
        let mut results = Vec::new();

        // Simulate scatter-gather (in production, use mesh channels)
        for target in &self.targets {
            let target_start = std::time::Instant::now();
            // Each target would be dispatched via TriLaneRouter.send()
            results.push(ScatterResult {
                service: target.service.clone(),
                success: true,
                response: Some(Bytes::from(format!("response from {}", target.service))),
                error: None,
                latency_ns: target_start.elapsed().as_nanos() as u64,
            });
        }

        GatherResult {
            total_targets: self.targets.len(),
            responses_received: results.len(),
            total_latency_ns: start.elapsed().as_nanos() as u64,
            results,
        }
    }

    pub fn target_count(&self) -> usize {
        self.targets.len()
    }
}

impl Default for ScatterGather {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a scatter-gather operation.
#[derive(Debug, Clone)]
pub struct GatherResult {
    pub total_targets: usize,
    pub responses_received: usize,
    pub total_latency_ns: u64,
    pub results: Vec<ScatterResult>,
}

impl GatherResult {
    /// Check if all targets responded successfully.
    pub fn all_success(&self) -> bool {
        self.results.iter().all(|r| r.success)
    }

    /// Get only successful responses.
    pub fn successes(&self) -> Vec<&ScatterResult> {
        self.results.iter().filter(|r| r.success).collect()
    }

    /// Get only failed responses.
    pub fn failures(&self) -> Vec<&ScatterResult> {
        self.results.iter().filter(|r| !r.success).collect()
    }
}
