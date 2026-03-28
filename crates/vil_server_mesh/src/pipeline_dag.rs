// =============================================================================
// VIL Server Mesh — Pipeline DAG Engine
// =============================================================================
//
// Executes directed acyclic graph (DAG) pipelines of service handlers.
// Each node in the DAG is a service handler that processes data and
// passes it to the next node(s) via Tri-Lane channels.
//
// Example DAG:
//   ingress → validator → [enricher, logger] → aggregator → egress
//
// Features:
//   - Parallel execution of independent nodes
//   - Topological ordering for dependency resolution
//   - SHM data passing between nodes (zero-copy)
//   - Error propagation via Control Lane

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// A node in the pipeline DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    /// Unique node identifier
    pub id: String,
    /// Service/handler name
    pub handler: String,
    /// Nodes that this node depends on (inputs)
    pub depends_on: Vec<String>,
    /// Node configuration
    pub config: Option<serde_json::Value>,
}

/// Pipeline DAG definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDag {
    /// Pipeline name
    pub name: String,
    /// Nodes in the DAG
    pub nodes: Vec<DagNode>,
}

/// Execution plan — topologically sorted list of execution stages.
/// Nodes within the same stage can execute in parallel.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Stages to execute in order. Each stage is a set of parallel nodes.
    pub stages: Vec<Vec<String>>,
    /// Total node count
    pub node_count: usize,
}

impl PipelineDag {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
        }
    }

    /// Add a node to the DAG.
    pub fn add_node(&mut self, node: DagNode) {
        self.nodes.push(node);
    }

    /// Validate the DAG (check for cycles, missing dependencies).
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let node_ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();

        // Check for missing dependencies
        for node in &self.nodes {
            for dep in &node.depends_on {
                if !node_ids.contains(dep.as_str()) {
                    errors.push(format!(
                        "Node '{}' depends on '{}' which doesn't exist",
                        node.id, dep
                    ));
                }
            }
        }

        // Check for cycles via topological sort
        if errors.is_empty() {
            if self.topological_sort().is_none() {
                errors.push("DAG contains a cycle".to_string());
            }
        }

        // Check for duplicate node IDs
        let mut seen = HashSet::new();
        for node in &self.nodes {
            if !seen.insert(&node.id) {
                errors.push(format!("Duplicate node ID: '{}'", node.id));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Topological sort using Kahn's algorithm.
    /// Returns None if there's a cycle.
    fn topological_sort(&self) -> Option<Vec<String>> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in &self.nodes {
            in_degree.entry(&node.id).or_insert(0);
            for dep in &node.depends_on {
                adjacency.entry(dep.as_str()).or_default().push(&node.id);
                *in_degree.entry(&node.id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut sorted = Vec::new();

        while let Some(node) = queue.pop_front() {
            sorted.push(node.to_string());
            if let Some(dependents) = adjacency.get(node) {
                for &dep in dependents {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        if sorted.len() == self.nodes.len() {
            Some(sorted)
        } else {
            None // Cycle detected
        }
    }

    /// Generate an execution plan with parallel stages.
    pub fn plan(&self) -> Result<ExecutionPlan, String> {
        self.validate().map_err(|errs| errs.join("; "))?;

        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in &self.nodes {
            in_degree.entry(&node.id).or_insert(0);
            for dep in &node.depends_on {
                adjacency.entry(dep.as_str()).or_default().push(&node.id);
                *in_degree.entry(&node.id).or_insert(0) += 1;
            }
        }

        let mut stages = Vec::new();
        let mut remaining: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();

        while !remaining.is_empty() {
            // Find all nodes with in_degree == 0 among remaining
            let stage: Vec<String> = remaining
                .iter()
                .filter(|&&id| *in_degree.get(id).unwrap_or(&0) == 0)
                .map(|&id| id.to_string())
                .collect();

            if stage.is_empty() {
                return Err("Unable to resolve DAG — possible cycle".to_string());
            }

            // Remove stage nodes and update in_degrees
            for id in &stage {
                remaining.remove(id.as_str());
                if let Some(dependents) = adjacency.get(id.as_str()) {
                    for &dep in dependents {
                        if let Some(deg) = in_degree.get_mut(dep) {
                            *deg = deg.saturating_sub(1);
                        }
                    }
                }
            }

            stages.push(stage);
        }

        Ok(ExecutionPlan {
            node_count: self.nodes.len(),
            stages,
        })
    }

    /// Get entry nodes (no dependencies — DAG roots).
    pub fn entry_nodes(&self) -> Vec<&DagNode> {
        self.nodes
            .iter()
            .filter(|n| n.depends_on.is_empty())
            .collect()
    }

    /// Get exit nodes (no dependents — DAG leaves).
    pub fn exit_nodes(&self) -> Vec<&DagNode> {
        let has_dependents: HashSet<&str> = self
            .nodes
            .iter()
            .flat_map(|n| n.depends_on.iter().map(|d| d.as_str()))
            .collect();

        self.nodes
            .iter()
            .filter(|n| !has_dependents.contains(n.id.as_str()))
            .collect()
    }
}
