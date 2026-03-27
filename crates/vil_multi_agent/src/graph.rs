//! AgentGraph — directed acyclic graph of agents.

use std::collections::HashMap;
use std::sync::Arc;

use crate::agent_node::{AgentNode, AgentRunnable};

/// Error returned when building or validating a graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// An edge references an agent name that was never added.
    UnknownAgent(String),
    /// Duplicate agent name.
    DuplicateAgent(String),
    /// The graph is empty (no agents).
    EmptyGraph,
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownAgent(name) => write!(f, "unknown agent: {}", name),
            Self::DuplicateAgent(name) => write!(f, "duplicate agent: {}", name),
            Self::EmptyGraph => write!(f, "graph has no agents"),
        }
    }
}

impl std::error::Error for GraphError {}

/// A DAG of agent nodes with directed edges.
#[derive(Debug, Clone)]
pub struct AgentGraph {
    /// Ordered list of agent nodes.
    pub nodes: Vec<AgentNode>,
    /// Directed edges (from, to).
    pub edges: Vec<(String, String)>,
}

impl AgentGraph {
    /// Create a builder for constructing an agent graph.
    pub fn builder() -> AgentGraphBuilder {
        AgentGraphBuilder::new()
    }

    /// Number of agents in the graph.
    pub fn agent_count(&self) -> usize {
        self.nodes.len()
    }

    /// Return root agents (no upstream).
    pub fn roots(&self) -> Vec<&AgentNode> {
        self.nodes.iter().filter(|n| n.is_root()).collect()
    }

    /// Return leaf agents (no downstream).
    pub fn leaves(&self) -> Vec<&AgentNode> {
        self.nodes.iter().filter(|n| n.is_leaf()).collect()
    }

    /// Look up a node by name.
    pub fn get(&self, name: &str) -> Option<&AgentNode> {
        self.nodes.iter().find(|n| n.name == name)
    }

    /// Return the downstream node names for a given agent.
    pub fn downstream_of(&self, name: &str) -> Vec<&str> {
        self.get(name)
            .map(|n| n.downstream.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Return the upstream node names for a given agent.
    pub fn upstream_of(&self, name: &str) -> Vec<&str> {
        self.get(name)
            .map(|n| n.upstream.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Topological ordering of agent names (BFS from roots).
    pub fn topological_order(&self) -> Vec<String> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for n in &self.nodes {
            in_degree.entry(&n.name).or_insert(0);
            for d in &n.downstream {
                *in_degree.entry(d.as_str()).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&name, _)| name)
            .collect();
        queue.sort(); // deterministic order

        let mut order = Vec::new();
        while let Some(name) = queue.pop() {
            order.push(name.to_string());
            if let Some(node) = self.get(name) {
                for d in &node.downstream {
                    if let Some(deg) = in_degree.get_mut(d.as_str()) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(d.as_str());
                            queue.sort();
                        }
                    }
                }
            }
        }
        order
    }
}

/// Fluent builder for constructing an `AgentGraph`.
pub struct AgentGraphBuilder {
    agents: HashMap<String, (String, Arc<dyn AgentRunnable>)>,
    edges: Vec<(String, String)>,
}

impl AgentGraphBuilder {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// Add an agent with the given name. Role defaults to the name.
    pub fn agent(
        mut self,
        name: impl Into<String>,
        agent: Arc<dyn AgentRunnable>,
    ) -> Self {
        let n: String = name.into();
        self.agents.insert(n.clone(), (n.clone(), agent));
        self
    }

    /// Add an agent with an explicit role.
    pub fn agent_with_role(
        mut self,
        name: impl Into<String>,
        role: impl Into<String>,
        agent: Arc<dyn AgentRunnable>,
    ) -> Self {
        let n: String = name.into();
        self.agents.insert(n.clone(), (role.into(), agent));
        self
    }

    /// Add a directed edge from one agent to another.
    pub fn edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.edges.push((from.into(), to.into()));
        self
    }

    /// Validate and build the graph.
    pub fn build(self) -> Result<AgentGraph, GraphError> {
        if self.agents.is_empty() {
            return Err(GraphError::EmptyGraph);
        }

        // Validate edges reference known agents.
        for (from, to) in &self.edges {
            if !self.agents.contains_key(from) {
                return Err(GraphError::UnknownAgent(from.clone()));
            }
            if !self.agents.contains_key(to) {
                return Err(GraphError::UnknownAgent(to.clone()));
            }
        }

        // Build nodes with upstream/downstream populated.
        let mut nodes: HashMap<String, AgentNode> = self
            .agents
            .into_iter()
            .map(|(name, (role, agent))| {
                (name.clone(), AgentNode::new(name, role, agent))
            })
            .collect();

        for (from, to) in &self.edges {
            if let Some(n) = nodes.get_mut(from) {
                n.downstream.push(to.clone());
            }
            if let Some(n) = nodes.get_mut(to) {
                n.upstream.push(from.clone());
            }
        }

        let mut node_list: Vec<AgentNode> = nodes.into_values().collect();
        node_list.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(AgentGraph {
            nodes: node_list,
            edges: self.edges,
        })
    }
}

impl Default for AgentGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_node::AgentRunnable;
    use async_trait::async_trait;

    struct MockAgent(String);

    #[async_trait]
    impl AgentRunnable for MockAgent {
        async fn run(&self, _input: &str) -> Result<String, String> {
            Ok(self.0.clone())
        }
    }

    fn mock(name: &str) -> Arc<dyn AgentRunnable> {
        Arc::new(MockAgent(format!("output-{}", name)))
    }

    #[test]
    fn test_single_agent_graph() {
        let graph = AgentGraph::builder()
            .agent("solo", mock("solo"))
            .build()
            .unwrap();

        assert_eq!(graph.agent_count(), 1);
        assert_eq!(graph.roots().len(), 1);
        assert_eq!(graph.leaves().len(), 1);
        assert_eq!(graph.roots()[0].name, "solo");
    }

    #[test]
    fn test_linear_chain() {
        let graph = AgentGraph::builder()
            .agent("planner", mock("planner"))
            .agent("executor", mock("executor"))
            .agent("reviewer", mock("reviewer"))
            .edge("planner", "executor")
            .edge("executor", "reviewer")
            .build()
            .unwrap();

        assert_eq!(graph.agent_count(), 3);
        assert_eq!(graph.roots().len(), 1);
        assert_eq!(graph.roots()[0].name, "planner");
        assert_eq!(graph.leaves().len(), 1);
        assert_eq!(graph.leaves()[0].name, "reviewer");
        assert_eq!(graph.downstream_of("planner"), vec!["executor"]);
        assert_eq!(graph.upstream_of("reviewer"), vec!["executor"]);
    }

    #[test]
    fn test_parallel_agents() {
        let graph = AgentGraph::builder()
            .agent("root", mock("root"))
            .agent("branch_a", mock("a"))
            .agent("branch_b", mock("b"))
            .agent("merger", mock("merger"))
            .edge("root", "branch_a")
            .edge("root", "branch_b")
            .edge("branch_a", "merger")
            .edge("branch_b", "merger")
            .build()
            .unwrap();

        assert_eq!(graph.agent_count(), 4);
        assert_eq!(graph.roots().len(), 1);
        assert_eq!(graph.leaves().len(), 1);
        let mut upstream = graph.upstream_of("merger");
        upstream.sort();
        assert_eq!(upstream, vec!["branch_a", "branch_b"]);
    }

    #[test]
    fn test_empty_graph() {
        let result = AgentGraph::builder().build();
        assert_eq!(result.unwrap_err(), GraphError::EmptyGraph);
    }

    #[test]
    fn test_agent_count() {
        let graph = AgentGraph::builder()
            .agent("a", mock("a"))
            .agent("b", mock("b"))
            .agent("c", mock("c"))
            .build()
            .unwrap();
        assert_eq!(graph.agent_count(), 3);
    }

    #[test]
    fn test_edge_validation_unknown_agent() {
        let result = AgentGraph::builder()
            .agent("a", mock("a"))
            .edge("a", "unknown")
            .build();
        assert_eq!(result.unwrap_err(), GraphError::UnknownAgent("unknown".into()));
    }

    #[test]
    fn test_topological_order() {
        let graph = AgentGraph::builder()
            .agent("planner", mock("planner"))
            .agent("executor", mock("executor"))
            .agent("reviewer", mock("reviewer"))
            .edge("planner", "executor")
            .edge("executor", "reviewer")
            .build()
            .unwrap();

        let order = graph.topological_order();
        assert_eq!(order, vec!["planner", "executor", "reviewer"]);
    }
}
