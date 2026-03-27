//! Orchestrator — executes the multi-agent graph.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing;
use vil_macros::VilAiEvent;

use crate::channel::AgentMessage;
use crate::config::MultiAgentConfig;
use crate::graph::AgentGraph;

/// Outcome of a full orchestrator run.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub struct OrchestratorResult {
    /// The final combined answer from leaf agents.
    pub final_answer: String,
    /// Per-agent outputs: `(agent_name, output)`.
    pub agent_outputs: Vec<(String, String)>,
    /// Total wall-clock time in milliseconds.
    pub total_ms: u64,
    /// Number of messages exchanged between agents.
    pub messages_exchanged: usize,
}

/// Error from orchestrator execution.
#[derive(Debug)]
pub enum OrchestratorError {
    /// An agent returned an error.
    AgentFailed { agent: String, reason: String },
    /// The graph is empty or invalid.
    InvalidGraph(String),
    /// A channel send/receive failed.
    ChannelError(String),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentFailed { agent, reason } => {
                write!(f, "agent '{}' failed: {}", agent, reason)
            }
            Self::InvalidGraph(msg) => write!(f, "invalid graph: {}", msg),
            Self::ChannelError(msg) => write!(f, "channel error: {}", msg),
        }
    }
}

impl std::error::Error for OrchestratorError {}

/// Orchestrator drives execution of an `AgentGraph`.
///
/// The execution flow:
/// 1. Find root agents (no upstream dependencies).
/// 2. Run root agents with the initial query.
/// 3. Pass each agent's output to its downstream agents as context.
/// 4. Continue until leaf agents produce output.
/// 5. Collect and return all results.
pub struct Orchestrator {
    pub graph: AgentGraph,
    pub config: MultiAgentConfig,
    /// Internal channels keyed by `"from->to"`.
    channels: HashMap<String, mpsc::Sender<AgentMessage>>,
}

impl Orchestrator {
    /// Create a new orchestrator for the given graph.
    pub fn new(graph: AgentGraph) -> Self {
        Self {
            graph,
            config: MultiAgentConfig::default(),
            channels: HashMap::new(),
        }
    }

    /// Create with explicit config.
    pub fn with_config(graph: AgentGraph, config: MultiAgentConfig) -> Self {
        Self {
            graph,
            config,
            channels: HashMap::new(),
        }
    }

    /// Execute the graph with the given initial query (dry-run).
    ///
    /// This runs every agent in topological order, passing outputs downstream.
    /// Agents are invoked via `AgentRunnable::run()`, making it easy to inject
    /// mocks for testing.
    pub async fn run(&mut self, initial_query: &str) -> Result<OrchestratorResult, OrchestratorError> {
        let start = std::time::Instant::now();
        let topo = self.graph.topological_order();

        if topo.is_empty() {
            return Err(OrchestratorError::InvalidGraph("empty graph".into()));
        }

        // Stores the output of each agent.
        let mut outputs: HashMap<String, String> = HashMap::new();
        let mut messages_exchanged: usize = 0;

        // Set up internal channel bookkeeping.
        self.channels.clear();

        for name in &topo {
            let node = self
                .graph
                .get(name)
                .ok_or_else(|| OrchestratorError::InvalidGraph(format!("missing node: {}", name)))?;

            // Build input for this agent.
            let input = if node.is_root() {
                // Root agents receive the initial query directly.
                initial_query.to_string()
            } else {
                // Non-root agents receive concatenated upstream outputs.
                let mut parts = Vec::new();
                for up in &node.upstream {
                    if let Some(out) = outputs.get(up) {
                        parts.push(format!("[{}]: {}", up, out));
                        messages_exchanged += 1;
                    }
                }
                parts.join("\n")
            };

            tracing::debug!(agent = %name, input_len = input.len(), "running agent");

            // Execute the agent.
            let output = node
                .agent
                .run(&input)
                .await
                .map_err(|reason| OrchestratorError::AgentFailed {
                    agent: name.clone(),
                    reason,
                })?;

            outputs.insert(name.clone(), output);
        }

        // Collect agent outputs (in topological order).
        let agent_outputs: Vec<(String, String)> = topo
            .iter()
            .filter_map(|name| outputs.get(name).map(|o| (name.clone(), o.clone())))
            .collect();

        // Final answer is the concatenation of leaf agent outputs.
        let leaves = self.graph.leaves();
        let final_answer = leaves
            .iter()
            .filter_map(|leaf| outputs.get(&leaf.name))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        let total_ms = start.elapsed().as_millis() as u64;

        Ok(OrchestratorResult {
            final_answer,
            agent_outputs,
            total_ms,
            messages_exchanged,
        })
    }

    /// Dry-run: execute the graph with mock passthrough.
    ///
    /// Identical to `run()` but explicitly named for clarity in tests.
    pub async fn dry_run(&mut self, initial_query: &str) -> Result<OrchestratorResult, OrchestratorError> {
        self.run(initial_query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_node::AgentRunnable;
    use crate::graph::AgentGraph;
    use async_trait::async_trait;
    use std::sync::Arc;

    /// Mock agent that prefixes input with its name.
    struct MockAgent(String);

    #[async_trait]
    impl AgentRunnable for MockAgent {
        async fn run(&self, input: &str) -> Result<String, String> {
            Ok(format!("[{}] processed: {}", self.0, input))
        }
    }

    fn mock(name: &str) -> Arc<dyn AgentRunnable> {
        Arc::new(MockAgent(name.to_string()))
    }

    #[tokio::test]
    async fn test_orchestrator_single_agent() {
        let graph = AgentGraph::builder()
            .agent("solo", mock("solo"))
            .build()
            .unwrap();

        let mut orch = Orchestrator::new(graph);
        let result = orch.run("hello").await.unwrap();

        assert!(result.final_answer.contains("[solo] processed: hello"));
        assert_eq!(result.agent_outputs.len(), 1);
        assert_eq!(result.messages_exchanged, 0);
    }

    #[tokio::test]
    async fn test_orchestrator_linear_chain() {
        let graph = AgentGraph::builder()
            .agent("planner", mock("planner"))
            .agent("executor", mock("executor"))
            .agent("reviewer", mock("reviewer"))
            .edge("planner", "executor")
            .edge("executor", "reviewer")
            .build()
            .unwrap();

        let mut orch = Orchestrator::new(graph);
        let result = orch.run("write code").await.unwrap();

        // Final answer is from the reviewer (leaf).
        assert!(result.final_answer.contains("[reviewer]"));
        assert_eq!(result.agent_outputs.len(), 3);
        assert!(result.messages_exchanged >= 2);
    }

    #[tokio::test]
    async fn test_orchestrator_parallel_merge() {
        let graph = AgentGraph::builder()
            .agent("root", mock("root"))
            .agent("branch_a", mock("branch_a"))
            .agent("branch_b", mock("branch_b"))
            .agent("merger", mock("merger"))
            .edge("root", "branch_a")
            .edge("root", "branch_b")
            .edge("branch_a", "merger")
            .edge("branch_b", "merger")
            .build()
            .unwrap();

        let mut orch = Orchestrator::new(graph);
        let result = orch.run("query").await.unwrap();

        assert!(result.final_answer.contains("[merger]"));
        // merger receives from branch_a and branch_b.
        assert_eq!(result.agent_outputs.len(), 4);
    }

    #[tokio::test]
    async fn test_orchestrator_result_contains_all_agents() {
        let graph = AgentGraph::builder()
            .agent("a", mock("a"))
            .agent("b", mock("b"))
            .agent("c", mock("c"))
            .edge("a", "b")
            .edge("b", "c")
            .build()
            .unwrap();

        let mut orch = Orchestrator::new(graph);
        let result = orch.run("test").await.unwrap();

        let names: Vec<&str> = result.agent_outputs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"c"));
    }

    #[tokio::test]
    async fn test_dry_run() {
        let graph = AgentGraph::builder()
            .agent("x", mock("x"))
            .build()
            .unwrap();

        let mut orch = Orchestrator::new(graph);
        let result = orch.dry_run("ping").await.unwrap();
        assert!(result.final_answer.contains("ping"));
    }

    #[tokio::test]
    async fn test_message_passing_content() {
        let graph = AgentGraph::builder()
            .agent("producer", mock("producer"))
            .agent("consumer", mock("consumer"))
            .edge("producer", "consumer")
            .build()
            .unwrap();

        let mut orch = Orchestrator::new(graph);
        let result = orch.run("data").await.unwrap();

        // consumer should have received producer's output.
        let consumer_output = result
            .agent_outputs
            .iter()
            .find(|(n, _)| n == "consumer")
            .unwrap();
        assert!(consumer_output.1.contains("[producer]"));
    }
}
