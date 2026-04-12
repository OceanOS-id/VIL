//! VILW Graph — compiled workflow representation for VIL runtime.
//! Simpler than VFlow's VWFC — bincode serialized, no zero-copy pointer math.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VilwGraph {
    pub id: String,
    pub name: String,
    pub nodes: Vec<VilwNode>,
    pub edges: Vec<VilwEdge>,
    pub variables: Vec<String>,
    pub entry_node: usize,
    pub durability_default: String,
    /// Webhook route (from trigger config)
    pub webhook_route: Option<String>,
    /// HTTP method for webhook (GET, POST, PUT, DELETE). Default: POST.
    pub webhook_method: String,
    /// Trigger type
    pub trigger_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VilwNode {
    pub id: String,
    pub kind: NodeKind,
    pub output_variable: Option<String>,
    pub durability: Option<String>,
    /// Serialized config (JSON). Depends on kind.
    pub config: serde_json::Value,
    /// Pre-compiled mappings
    pub mappings: Vec<CompiledMapping>,
    /// Compensation config (for saga)
    pub compensation: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Trigger,
    Connector,
    Transform,
    EndTrigger,
    End,
    VilRules,
    Parallel,
    Join,
    ExclusiveGateway,
    InclusiveGateway,
    LoopWhile,
    LoopForEach,
    LoopRepeat,
    ErrorBoundary,
    Function,
    Sidecar,
    SubWorkflow,
    HumanTask,
    NativeCode,
    Noop,
}

impl NodeKind {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "trigger" => Self::Trigger,
            "connector" => Self::Connector,
            "transform" => Self::Transform,
            "endtrigger" => Self::EndTrigger,
            "end" => Self::End,
            "vrule" | "vilrules" => Self::VilRules,
            "parallel" | "fork" => Self::Parallel,
            "join" => Self::Join,
            "exclusive" | "exclusivegateway" => Self::ExclusiveGateway,
            "inclusive" | "inclusivegateway" => Self::InclusiveGateway,
            "loopwhile" => Self::LoopWhile,
            "loopforeach" => Self::LoopForEach,
            "looprepeat" => Self::LoopRepeat,
            "errorboundary" => Self::ErrorBoundary,
            "function" => Self::Function,
            "sidecar" => Self::Sidecar,
            "subworkflow" => Self::SubWorkflow,
            "humantask" => Self::HumanTask,
            "nativecode" | "native_code" | "code" => Self::NativeCode,
            _ => Self::Noop,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VilwEdge {
    pub from_idx: usize,
    pub to_idx: usize,
    pub condition: Option<String>,
    pub priority: i8,
    pub detached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledMapping {
    pub target: String,
    pub language: String,
    /// For literal/spv1: the source string.
    /// For v-cel: the expression string (evaluated at runtime by vil_expr).
    /// For vil_query: pre-compiled SQL + param_refs.
    pub source: String,
    /// Pre-compiled SQL for vil_query (None for other languages).
    pub compiled_sql: Option<String>,
    /// Param refs for vil_query (variable paths to resolve at runtime).
    pub param_refs: Option<Vec<String>>,
}

impl VilwGraph {
    pub fn node_index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    pub fn outgoing_edges(&self, node_idx: usize) -> Vec<&VilwEdge> {
        self.edges.iter().filter(|e| e.from_idx == node_idx).collect()
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(data).map_err(|e| format!("VILW deserialize: {}", e))
    }
}
