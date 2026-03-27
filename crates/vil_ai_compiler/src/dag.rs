//! DAG representation and builder for AI pipelines.

use std::collections::HashMap;

use crate::node::PipelineNode;

/// A directed acyclic graph representing an AI pipeline.
#[derive(Debug, Clone)]
pub struct PipelineDag {
    /// Nodes: `(id, node)`.
    pub nodes: Vec<(String, PipelineNode)>,
    /// Edges: `(from_index, to_index)`.
    pub edges: Vec<(usize, usize)>,
}

/// Error returned when building or validating a DAG.
#[derive(Debug, Clone, PartialEq)]
pub enum DagError {
    /// A node id referenced in an edge does not exist.
    NodeNotFound(String),
    /// The graph contains a cycle.
    CycleDetected,
    /// Duplicate node id.
    DuplicateNode(String),
}

impl std::fmt::Display for DagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DagError::NodeNotFound(id) => write!(f, "node not found: {id}"),
            DagError::CycleDetected => write!(f, "cycle detected in DAG"),
            DagError::DuplicateNode(id) => write!(f, "duplicate node id: {id}"),
        }
    }
}

impl std::error::Error for DagError {}

impl PipelineDag {
    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns the index of a node by id, if present.
    pub fn node_index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|(nid, _)| nid == id)
    }

    /// Returns the direct dependencies (predecessors) of a node index.
    pub fn predecessors(&self, idx: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter_map(|&(from, to)| if to == idx { Some(from) } else { None })
            .collect()
    }

    /// Returns the direct successors of a node index.
    pub fn successors(&self, idx: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter_map(|&(from, to)| if from == idx { Some(to) } else { None })
            .collect()
    }
}

/// Fluent builder for `PipelineDag`.
#[derive(Debug, Default)]
pub struct DagBuilder {
    nodes: Vec<(String, PipelineNode)>,
    edges: Vec<(String, String)>,
}

impl DagBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node with the given id.
    pub fn node(mut self, id: impl Into<String>, node: PipelineNode) -> Self {
        self.nodes.push((id.into(), node));
        self
    }

    /// Add a directed edge from `from` to `to` (both are node ids).
    pub fn edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.edges.push((from.into(), to.into()));
        self
    }

    /// Build the DAG, resolving string edge references to indices.
    pub fn build(self) -> Result<PipelineDag, DagError> {
        // Check for duplicate node ids.
        let mut seen = HashMap::new();
        for (i, (id, _)) in self.nodes.iter().enumerate() {
            if seen.insert(id.clone(), i).is_some() {
                return Err(DagError::DuplicateNode(id.clone()));
            }
        }

        // Resolve edges.
        let mut edges = Vec::with_capacity(self.edges.len());
        for (from_id, to_id) in &self.edges {
            let from = *seen
                .get(from_id)
                .ok_or_else(|| DagError::NodeNotFound(from_id.clone()))?;
            let to = *seen
                .get(to_id)
                .ok_or_else(|| DagError::NodeNotFound(to_id.clone()))?;
            edges.push((from, to));
        }

        let dag = PipelineDag {
            nodes: self.nodes,
            edges,
        };

        // Cycle check via topological sort attempt.
        if has_cycle(&dag) {
            return Err(DagError::CycleDetected);
        }

        Ok(dag)
    }
}

/// Kahn's algorithm cycle detection.
fn has_cycle(dag: &PipelineDag) -> bool {
    let n = dag.nodes.len();
    let mut in_degree = vec![0usize; n];
    for &(_, to) in &dag.edges {
        in_degree[to] += 1;
    }

    let mut queue: Vec<usize> = in_degree
        .iter()
        .enumerate()
        .filter_map(|(i, &d)| if d == 0 { Some(i) } else { None })
        .collect();

    let mut visited = 0usize;
    while let Some(node) = queue.pop() {
        visited += 1;
        for &(from, to) in &dag.edges {
            if from == node {
                in_degree[to] -= 1;
                if in_degree[to] == 0 {
                    queue.push(to);
                }
            }
        }
    }

    visited != n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::PipelineNode;

    fn embed_node() -> PipelineNode {
        PipelineNode::Embed {
            model: "m".into(),
            dimensions: 128,
        }
    }

    fn search_node() -> PipelineNode {
        PipelineNode::Search {
            index: "idx".into(),
            top_k: 10,
        }
    }

    #[test]
    fn test_build_simple() {
        let dag = DagBuilder::new()
            .node("a", embed_node())
            .node("b", search_node())
            .edge("a", "b")
            .build()
            .unwrap();
        assert_eq!(dag.node_count(), 2);
        assert_eq!(dag.edge_count(), 1);
    }

    #[test]
    fn test_duplicate_node() {
        let res = DagBuilder::new()
            .node("a", embed_node())
            .node("a", search_node())
            .build();
        assert_eq!(res.unwrap_err(), DagError::DuplicateNode("a".into()));
    }

    #[test]
    fn test_node_not_found() {
        let res = DagBuilder::new()
            .node("a", embed_node())
            .edge("a", "missing")
            .build();
        assert_eq!(res.unwrap_err(), DagError::NodeNotFound("missing".into()));
    }

    #[test]
    fn test_cycle_detected() {
        let res = DagBuilder::new()
            .node("a", embed_node())
            .node("b", search_node())
            .edge("a", "b")
            .edge("b", "a")
            .build();
        assert_eq!(res.unwrap_err(), DagError::CycleDetected);
    }

    #[test]
    fn test_predecessors_successors() {
        let dag = DagBuilder::new()
            .node("a", embed_node())
            .node("b", search_node())
            .edge("a", "b")
            .build()
            .unwrap();
        assert_eq!(dag.predecessors(1), vec![0]);
        assert_eq!(dag.successors(0), vec![1]);
    }
}
