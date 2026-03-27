//! AgentNode — wrapper around an agent with a role inside the graph.

use std::sync::Arc;
use async_trait::async_trait;

/// Trait abstracting what an "agent" can do inside the multi-agent graph.
///
/// This is intentionally decoupled from `vil_agent::Agent` so that users can
/// plug in mock implementations for testing without requiring a real LLM.
#[async_trait]
pub trait AgentRunnable: Send + Sync {
    /// Execute the agent with the given input and return a textual response.
    async fn run(&self, input: &str) -> Result<String, String>;
}

/// A node in the agent graph.
///
/// Each node has a unique `name`, a human-readable `role`, and a reference to
/// an `AgentRunnable` that performs the actual work. The `upstream` and
/// `downstream` vectors record the directed edges.
#[derive(Clone)]
pub struct AgentNode {
    /// Unique identifier within the graph.
    pub name: String,
    /// Human-readable role (e.g. "planner", "reviewer").
    pub role: String,
    /// The underlying agent implementation.
    pub agent: Arc<dyn AgentRunnable>,
    /// Names of agents that feed into this node.
    pub upstream: Vec<String>,
    /// Names of agents that receive output from this node.
    pub downstream: Vec<String>,
}

impl AgentNode {
    /// Create a new agent node.
    pub fn new(
        name: impl Into<String>,
        role: impl Into<String>,
        agent: Arc<dyn AgentRunnable>,
    ) -> Self {
        Self {
            name: name.into(),
            role: role.into(),
            agent,
            upstream: Vec::new(),
            downstream: Vec::new(),
        }
    }

    /// Returns `true` if this node has no upstream dependencies (root node).
    pub fn is_root(&self) -> bool {
        self.upstream.is_empty()
    }

    /// Returns `true` if this node has no downstream consumers (leaf node).
    pub fn is_leaf(&self) -> bool {
        self.downstream.is_empty()
    }
}

impl std::fmt::Debug for AgentNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentNode")
            .field("name", &self.name)
            .field("role", &self.role)
            .field("upstream", &self.upstream)
            .field("downstream", &self.downstream)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyAgent;

    #[async_trait]
    impl AgentRunnable for DummyAgent {
        async fn run(&self, input: &str) -> Result<String, String> {
            Ok(format!("echo: {}", input))
        }
    }

    #[test]
    fn test_agent_node_root_leaf() {
        let node = AgentNode::new("a", "tester", Arc::new(DummyAgent));
        assert!(node.is_root());
        assert!(node.is_leaf());
    }

    #[test]
    fn test_agent_node_with_edges() {
        let mut node = AgentNode::new("b", "worker", Arc::new(DummyAgent));
        node.upstream.push("a".into());
        node.downstream.push("c".into());
        assert!(!node.is_root());
        assert!(!node.is_leaf());
    }
}
