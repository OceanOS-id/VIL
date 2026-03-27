//! VIL Workflow Manifest v6.0.0
//!
//! Single source of truth for the entire application.
//! Compiles through: YAML → manifest parse → WorkflowBuilder → WorkflowIR → codegen → binary

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════════
// Root manifest
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManifest {
    pub vil_version: String,
    pub name: String,
    pub port: u16,
    #[serde(default)]
    pub metrics_port: Option<u16>,
    #[serde(default)]
    pub prefix: Option<String>,
    /// Token type: "shm" (default, zero-copy, multi-pipeline) or "generic" (single pipeline, simpler)
    #[serde(default = "default_shm")]
    pub token: String,

    // ── Server-mode fields ──────────────────────────────────────────────────
    #[serde(default)]
    pub endpoints: Vec<EndpointManifest>,

    // ── Shared fields (both modes) ──────────────────────────────────────────
    #[serde(default)]
    pub state: Option<StateManifest>,
    #[serde(default)]
    pub mesh: Option<MeshManifest>,
    #[serde(default)]
    pub errors: Vec<ErrorVariant>,
    #[serde(default)]
    pub semantic_types: Vec<SemanticTypeManifest>,
    #[serde(default)]
    pub failover: Option<FailoverManifest>,
    #[serde(default)]
    pub sse_events: Vec<SseEventManifest>,
    #[serde(default)]
    pub ws_events: Vec<WsEventManifest>,

    // ── Workflow-mode fields (N-node DAG topology) ──────────────────────────
    #[serde(default)]
    pub messages: Vec<MessageManifest>,
    #[serde(default)]
    pub nodes: HashMap<String, NodeManifest>,
    #[serde(default, rename = "routes")]
    pub workflow_routes: Vec<RouteManifest>,
    #[serde(default)]
    pub topology: Option<TopologyManifest>,
    #[serde(default)]
    pub workflows: HashMap<String, WorkflowDagManifest>,

    // ── WASM function modules ───────────────────────────────────────────────
    #[serde(default)]
    pub vil_wasm: Vec<WasmModuleManifest>,

    // ── ORM/DB sections ─────────────────────────────────────────────────────
    #[serde(default)]
    pub database: Option<DatabaseManifest>,
    #[serde(default)]
    pub entities: Vec<EntityManifest>,
    #[serde(default)]
    pub cache: Option<CacheManifest>,
    #[serde(default)]
    pub message_queue: Option<MqManifest>,

    // ── Sidecar definitions ─────────────────────────────────────────────
    #[serde(default)]
    pub sidecars: Vec<SidecarManifest>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Server-mode types
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointManifest {
    pub method: String,
    pub path: String,
    pub handler: String,
    pub input: Option<SchemaManifest>,
    pub output: Option<SchemaManifest>,
    #[serde(default)]
    pub upstream: Option<UpstreamManifest>,
    #[serde(default = "default_exec_class")]
    pub exec_class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaManifest {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(default)]
    pub fields: Vec<FieldManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldManifest {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub items_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamManifest {
    #[serde(rename = "type")]
    pub upstream_type: String,
    pub url: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub body_template: Option<serde_yaml::Value>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Shared types
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateManifest {
    #[serde(rename = "type")]
    pub storage_type: String,
    #[serde(default)]
    pub fields: Vec<FieldManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshManifest {
    #[serde(default)]
    pub routes: Vec<MeshRouteManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshRouteManifest {
    pub from: String,
    pub to: String,
    pub lane: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorVariant {
    pub name: String,
    pub status: u16,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub retry: Option<bool>,
    #[serde(default)]
    pub fields: Vec<FieldManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SemanticTypeManifest {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub fields: Vec<FieldManifest>,
    #[serde(default)]
    pub variants: Vec<String>,
}

// ── Failover (Gap 3: condition enum) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FailoverManifest {
    #[serde(default)]
    pub entries: Vec<FailoverEntryManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverEntryManifest {
    pub primary: String,
    pub backup: String,
    #[serde(default = "default_failover_strategy")]
    pub strategy: String,
    #[serde(default)]
    pub condition: Option<FailoverCondition>,
}

/// Failover trigger condition — single variant, array, or expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FailoverCondition {
    Single(String),
    Multiple(Vec<String>),
    Expr { condition_expr: String },
}

// ── Events ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEventManifest {
    pub name: String,
    #[serde(default)]
    pub topic: Option<String>,
    pub fields: Vec<FieldManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEventManifest {
    pub name: String,
    #[serde(default)]
    pub topic: Option<String>,
    pub fields: Vec<FieldManifest>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Messages
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageManifest {
    pub name: String,
    #[serde(default = "default_message_kind")]
    pub kind: String,
    #[serde(default)]
    pub memory_class: Option<String>,
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub delivery: Option<String>,
    #[serde(default)]
    pub fields: Vec<FieldManifest>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Nodes
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeManifest {
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    /// SSE dialect: openai, anthropic, ollama, cohere, gemini, standard, custom
    #[serde(default)]
    pub dialect: Option<String>,
    /// Custom dialect: done marker string (e.g., "data: [END]")
    #[serde(default)]
    pub dialect_done_marker: Option<String>,
    /// Custom dialect: done event name (e.g., "stream_end")
    #[serde(default)]
    pub dialect_done_event: Option<String>,
    /// Custom dialect: done JSON field=value (e.g., "status=complete")
    #[serde(default)]
    pub dialect_done_json: Option<String>,
    #[serde(default)]
    pub json_tap: Option<String>,
    #[serde(default)]
    pub post_body: Option<serde_yaml::Value>,
    #[serde(default = "default_exec_class")]
    pub exec_class: String,
    /// Custom code: expr, handler, script, or wasm (replaces old `handler:` field)
    #[serde(default)]
    pub code: Option<CodeManifest>,
    #[serde(default)]
    pub ports: HashMap<String, PortManifest>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub gather: Option<GatherManifest>,
    /// Node-level config (for AI/DB built-in node types)
    #[serde(default)]
    pub config: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortManifest {
    pub direction: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub lane: Option<String>,
}

/// Scatter-gather configuration (Gap 4: full strategy detail).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatherManifest {
    pub strategy: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub quorum: Option<usize>,
    #[serde(default)]
    pub min_results: Option<usize>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Routes (Gap 6: routes carry NO code — only wiring properties)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteManifest {
    pub from: String,
    pub to: String,
    #[serde(default = "default_loan_write")]
    pub mode: String,
    /// Detached = non-blocking background branch (Spec Section 7)
    #[serde(default)]
    pub detach: Option<bool>,
    #[serde(default)]
    pub priority: Option<u8>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Code Activities (Spec Section 6 — 3 modes + wasm)
// ═══════════════════════════════════════════════════════════════════════════════

/// Custom code definition — unified across nodes and tasks.
/// Modes: expr, handler, script, wasm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeManifest {
    pub mode: String,
    // ── Mode: expr ──
    #[serde(default)]
    pub expr: Option<String>,
    // ── Mode: handler ──
    #[serde(default)]
    pub handler: Option<String>,
    #[serde(default = "default_true")]
    pub r#async: bool,
    #[serde(default)]
    pub state_access: Option<String>,
    // ── Mode: script ──
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub source_inline: Option<String>,
    #[serde(default)]
    pub sandbox: Option<SandboxManifest>,
    #[serde(default)]
    pub hot_reload: Option<bool>,
    // ── Mode: wasm ──
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub function: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxManifest {
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub max_memory_mb: Option<u64>,
    #[serde(default)]
    pub allow_net: Option<bool>,
    #[serde(default)]
    pub allow_fs: Option<bool>,
    #[serde(default)]
    pub max_output_size_kb: Option<u64>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Retry / Backoff (Gap 5: 3 levels — workflow, task, edge)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryManifest {
    #[serde(default)]
    pub max: Option<u32>,
    #[serde(default)]
    pub backoff: Option<String>,
    #[serde(default)]
    pub base_delay_ms: Option<u64>,
    #[serde(default)]
    pub max_delay_ms: Option<u64>,
    #[serde(default)]
    pub on: Option<Vec<String>>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Topology
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopologyManifest {
    #[serde(default)]
    pub hosts: HashMap<String, String>,
    #[serde(default)]
    pub placement: HashMap<String, PlacementManifest>,
    #[serde(default)]
    pub transport_override: Vec<TransportOverrideManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementManifest {
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportOverrideManifest {
    pub from: String,
    pub to: String,
    pub transport: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Workflow DAGs (Layer 2 inside Layer 1 nodes)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowDagManifest {
    #[serde(default)]
    pub trigger: Option<String>,
    /// Typed contract (for standalone workflows / call: composition)
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub tasks: Vec<TaskManifest>,
    #[serde(default)]
    pub branches: Vec<BranchManifest>,
    /// Workflow-level retry default
    #[serde(default)]
    pub retry: Option<RetryManifest>,
    /// Auto-trigger on completion
    #[serde(default)]
    pub on_complete: Option<OnCompleteManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnCompleteManifest {
    #[serde(default)]
    pub success: Option<String>,
    #[serde(default)]
    pub failure: Option<String>,
}

/// A single task in a workflow DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManifest {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type", default)]
    pub task_type: Option<String>,
    /// Deps: string (attached) or object (advanced with detach/on/retry)
    #[serde(default)]
    pub deps: Vec<DepSpec>,
    #[serde(default)]
    pub config: Option<serde_yaml::Value>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// Custom code (Gap 1: task-level script/expr/handler/wasm)
    #[serde(default)]
    pub code: Option<CodeManifest>,
    /// Call another workflow file
    #[serde(default)]
    pub call: Option<String>,
    /// Task-level retry override
    #[serde(default)]
    pub retry: Option<RetryManifest>,
}

/// Dependency spec — string (simple) or object (advanced).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DepSpec {
    Simple(String),
    Advanced {
        task: String,
        #[serde(default)]
        detach: Option<bool>,
        #[serde(default)]
        on: Option<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        retry: Option<RetryManifest>,
    },
}

/// Branch or Switch in a workflow DAG (Gap 2: Switch with cases).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchManifest {
    pub id: String,
    #[serde(rename = "type")]
    pub branch_type: String,
    // ── Branch fields ──
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub on_true: Option<String>,
    #[serde(default)]
    pub on_false: Option<String>,
    // ── Switch fields (Gap 2) ──
    #[serde(default)]
    pub expr: Option<String>,
    #[serde(default)]
    pub cases: Option<Vec<SwitchCaseManifest>>,
    #[serde(default)]
    pub default: Option<String>,
    // ── Common ──
    #[serde(default)]
    pub deps: Vec<DepSpec>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub code: Option<CodeManifest>,
    #[serde(default)]
    pub config: Option<serde_yaml::Value>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchCaseManifest {
    pub value: String,
    pub target: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// WASM function modules (Batch C)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModuleManifest {
    pub name: String,
    #[serde(default = "default_rust")]
    pub language: String,
    #[serde(default)]
    pub source_dir: Option<String>,
    #[serde(default)]
    pub wasm_path: Option<String>,
    pub functions: Vec<WasmFunctionManifest>,
    #[serde(default)]
    pub sandbox: Option<SandboxManifest>,
    #[serde(default)]
    pub pool_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmFunctionManifest {
    pub name: String,
    #[serde(default)]
    pub input: Option<serde_yaml::Value>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ORM / Database / Cache / MQ (Batch E)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseManifest {
    #[serde(default)]
    pub pools: HashMap<String, DbPoolManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbPoolManifest {
    pub driver: String,
    pub url: String,
    #[serde(default = "default_20")]
    pub max_connections: u32,
    #[serde(default = "default_5")]
    pub min_connections: u32,
    #[serde(default)]
    pub connect_timeout_secs: Option<u64>,
    #[serde(default)]
    pub idle_timeout_secs: Option<u64>,
    #[serde(default)]
    pub ssl_mode: Option<String>,
    #[serde(default)]
    pub services: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityManifest {
    pub name: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub table: Option<String>,
    pub fields: Vec<EntityFieldManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityFieldManifest {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub primary_key: Option<bool>,
    #[serde(default)]
    pub nullable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheManifest {
    pub backend: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub default_ttl_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqManifest {
    #[serde(default)]
    pub nats: Option<NatsMqManifest>,
    #[serde(default)]
    pub kafka: Option<KafkaMqManifest>,
    #[serde(default)]
    pub mqtt: Option<MqttMqManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsMqManifest {
    pub url: String,
    #[serde(default)]
    pub subjects: Vec<String>,
    #[serde(default)]
    pub credentials: Option<NatsCredentialsManifest>,
    #[serde(default)]
    pub tls: Option<bool>,
    #[serde(default)]
    pub max_reconnects: Option<u32>,
    #[serde(default)]
    pub jetstream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsCredentialsManifest {
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaMqManifest {
    pub brokers: Vec<String>,
    #[serde(default)]
    pub consumer_group: Option<String>,
    #[serde(default)]
    pub acks: Option<String>,
    #[serde(default)]
    pub security_protocol: Option<String>,
    #[serde(default)]
    pub sasl_mechanism: Option<String>,
    #[serde(default)]
    pub sasl_username: Option<String>,
    #[serde(default)]
    pub sasl_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttMqManifest {
    pub broker_url: String,
    #[serde(default = "default_mqtt_port")]
    pub port: u16,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub qos: Option<u8>,
    #[serde(default)]
    pub tls: Option<bool>,
    #[serde(default)]
    pub keepalive_secs: Option<u64>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Sidecar configuration
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarManifest {
    pub name: String,
    /// Runtime command: "python3", "go run", "java -jar", "node", etc.
    #[serde(default)]
    pub command: Option<String>,
    /// Script or binary path (relative to project root)
    #[serde(default)]
    pub script: Option<String>,
    /// Communication protocol: "uds" (default), "tcp"
    #[serde(default)]
    pub protocol: Option<String>,
    /// UDS socket path (auto-generated if not specified)
    #[serde(default)]
    pub socket: Option<String>,
    /// Methods this sidecar exposes
    #[serde(default)]
    pub methods: Vec<String>,
    /// SHM bridge buffer size
    #[serde(default)]
    pub shm_size: Option<String>,
    /// Request timeout in milliseconds
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// Connection pool size
    #[serde(default)]
    pub pool_size: Option<usize>,
    /// Max in-flight requests (backpressure)
    #[serde(default)]
    pub max_in_flight: Option<u64>,
    /// Health check interval in milliseconds
    #[serde(default)]
    pub health_interval_ms: Option<u64>,
    /// Authentication token for sidecar protocol
    #[serde(default)]
    pub auth_token: Option<String>,
    /// Failover configuration
    #[serde(default)]
    pub failover: Option<SidecarFailoverManifest>,
    /// Retry configuration
    #[serde(default)]
    pub retry: Option<RetryManifest>,
    /// Auto-restart on crash
    #[serde(default)]
    pub auto_restart: Option<bool>,
    /// Startup timeout in milliseconds
    #[serde(default)]
    pub startup_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarFailoverManifest {
    #[serde(default)]
    pub backup: Option<String>,
    #[serde(default)]
    pub fallback_wasm: Option<String>,
    #[serde(default)]
    pub failure_threshold: Option<u32>,
    #[serde(default)]
    pub cooldown_secs: Option<u32>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Default value functions
// ═══════════════════════════════════════════════════════════════════════════════

fn default_exec_class() -> String { "AsyncTask".into() }
fn default_failover_strategy() -> String { "immediate".into() }
fn default_message_kind() -> String { "message".into() }
fn default_loan_write() -> String { "LoanWrite".into() }
fn default_true() -> bool { true }
fn default_rust() -> String { "rust".into() }
fn default_shm() -> String { "shm".into() }
fn default_20() -> u32 { 20 }
fn default_5() -> u32 { 5 }
fn default_mqtt_port() -> u16 { 1883 }

// ═══════════════════════════════════════════════════════════════════════════════
// Impl
// ═══════════════════════════════════════════════════════════════════════════════

impl WorkflowManifest {
    pub fn manifest_mode(&self) -> &str {
        if !self.nodes.is_empty() { "workflow" } else { "server" }
    }

    pub fn is_workflow(&self) -> bool {
        self.manifest_mode() == "workflow"
    }

    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml).map_err(|e| format!("Failed to parse manifest YAML: {}", e))
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read manifest file '{}': {}", path, e))?;
        Self::from_yaml(&content)
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.is_empty() { errors.push("name is required".into()); }
        if self.port == 0 { errors.push("port must be > 0".into()); }

        match self.manifest_mode() {
            "workflow" => {
                if self.nodes.is_empty() {
                    errors.push("workflow mode requires at least one node".into());
                }

                // Validate route references
                for (i, route) in self.workflow_routes.iter().enumerate() {
                    let from_parts: Vec<&str> = route.from.splitn(2, '.').collect();
                    let to_parts: Vec<&str> = route.to.splitn(2, '.').collect();
                    if from_parts.len() != 2 {
                        errors.push(format!("route[{}]: 'from' must be node.port, got '{}'", i, route.from));
                    } else if !self.nodes.contains_key(from_parts[0]) {
                        errors.push(format!("route[{}]: node '{}' not found", i, from_parts[0]));
                    }
                    if to_parts.len() != 2 {
                        errors.push(format!("route[{}]: 'to' must be node.port, got '{}'", i, route.to));
                    } else if !self.nodes.contains_key(to_parts[0]) {
                        errors.push(format!("route[{}]: node '{}' not found", i, to_parts[0]));
                    }
                }

                // Validate workflow bindings
                for (wf_name, wf) in &self.workflows {
                    if !self.nodes.contains_key(wf_name) && wf.trigger.is_none() {
                        errors.push(format!("workflow '{}' not bound to any node and has no 'trigger:'", wf_name));
                    }
                    if let Some(trigger) = &wf.trigger {
                        if !self.nodes.contains_key(trigger) {
                            errors.push(format!("workflow '{}' trigger '{}' references unknown node", wf_name, trigger));
                        }
                    }
                }

                // Validate node types
                for (name, node) in &self.nodes {
                    match node.node_type.as_str() {
                        "http-sink" => {
                            if node.port.is_none() { errors.push(format!("node '{}': port required", name)); }
                            if node.path.is_none() { errors.push(format!("node '{}': path required", name)); }
                        }
                        "http-source" => {
                            if node.url.is_none() { errors.push(format!("node '{}': url required", name)); }
                        }
                        "transform" => {}
                        _ => {} // Allow AI/DB built-in types without validation here
                    }
                }
            }
            _ => {
                if self.endpoints.is_empty() { errors.push("at least one endpoint required".into()); }
                for (i, ep) in self.endpoints.iter().enumerate() {
                    if ep.method.is_empty() { errors.push(format!("endpoint[{}]: method required", i)); }
                    if ep.path.is_empty() { errors.push(format!("endpoint[{}]: path required", i)); }
                    if ep.handler.is_empty() { errors.push(format!("endpoint[{}]: handler required", i)); }
                }
            }
        }

        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

impl DepSpec {
    pub fn task_id(&self) -> &str {
        match self {
            DepSpec::Simple(s) => s,
            DepSpec::Advanced { task, .. } => task,
        }
    }

    pub fn is_detached(&self) -> bool {
        match self {
            DepSpec::Simple(_) => false,
            DepSpec::Advanced { detach, .. } => detach.unwrap_or(false),
        }
    }
}
