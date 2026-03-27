//! Core graph data model for VIL workflow visualization.
//!
//! VizGraph is the intermediate representation between the manifest and
//! any output format. It is fully serializable (JSON roundtrip) and serves
//! as the data API for the future egui-based IDE.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete visualization graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VizGraph {
    pub name: String,
    pub nodes: Vec<VizNode>,
    pub edges: Vec<VizEdge>,
    /// Workflow DAGs nested inside topology nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subgraphs: Vec<VizSubgraph>,
}

/// A node in the visualization graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VizNode {
    pub id: String,
    pub label: String,
    pub node_type: VizNodeType,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<VizPort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

/// Node type for visual styling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VizNodeType {
    Sink,
    Source,
    Transform,
    Task,
    Branch,
    Switch,
    Merge,
    Wasm,
}

impl VizNodeType {
    pub fn icon(&self) -> &'static str {
        match self {
            VizNodeType::Sink => "IN",
            VizNodeType::Source => "OUT",
            VizNodeType::Transform => "FN",
            VizNodeType::Task => "T",
            VizNodeType::Branch => "?",
            VizNodeType::Switch => "SW",
            VizNodeType::Merge => "M",
            VizNodeType::Wasm => "W",
        }
    }
}

/// An edge (route) in the visualization graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VizEdge {
    pub from_node: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_port: Option<String>,
    pub to_node: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_port: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    /// Detached edge = non-blocking background branch
    #[serde(default)]
    pub detach: bool,
}

/// A port on a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VizPort {
    pub name: String,
    pub direction: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<String>,
}

/// A subgraph representing a workflow DAG inside a topology node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VizSubgraph {
    pub parent_node: String,
    pub nodes: Vec<VizNode>,
    pub edges: Vec<VizEdge>,
}
