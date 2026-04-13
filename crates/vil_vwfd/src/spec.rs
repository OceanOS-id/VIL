//! VWFD v3.0 Schema — Shared spec between VIL and VFlow.
//! These structs define the YAML format. Identical to VFlow's vwfd.rs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct VwfdDocument {
    pub version: Option<String>,
    pub metadata: Option<VwfdMetadata>,
    pub spec: VwfdSpec,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VwfdMetadata {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub workflow_version: Option<serde_yaml::Value>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
    pub updated_at: Option<String>,
    /// State store type for execution state tracking.
    /// Values: "in_memory", "h2_in_memory", "redb", "postgres" (future).
    /// Default: none (stateless execution).
    pub state_store: Option<String>,
    /// State store path (for redb) or connection URL (for postgres).
    pub state_store_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VwfdSpec {
    pub activities: Vec<VwfdActivity>,
    pub controls: Option<Vec<VwfdControl>>,
    pub flows: Vec<VwfdFlow>,
    pub variables: Option<Vec<VwfdVariable>>,
    pub durability: Option<DurabilityConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DurabilityConfig {
    pub enabled: Option<bool>,
    pub default_mode: Option<String>,
    pub compensation_timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VwfdActivity {
    pub id: String,
    pub name: Option<String>,
    pub activity_type: String,
    pub description: Option<String>,
    pub output_variable: Option<String>,

    pub trigger_config: Option<TriggerConfig>,
    pub connector_config: Option<ConnectorConfig>,
    pub rule_config: Option<RuleConfig>,
    pub end_trigger_config: Option<EndTriggerConfig>,
    pub end_config: Option<EndConfig>,
    pub wasm_config: Option<WasmConfig>,
    pub sidecar_config: Option<SidecarActivityConfig>,
    pub sub_workflow_config: Option<SubWorkflowConfig>,
    pub human_task_config: Option<HumanTaskConfig>,
    pub code_config: Option<NativeCodeConfig>,

    pub input_mappings: Option<Vec<InputMapping>>,
    pub loop_config: Option<LoopConfig>,

    pub durability: Option<String>,
    pub compensation: Option<CompensationConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompensationConfig {
    pub connector_ref: Option<String>,
    pub operation: Option<String>,
    pub input_mappings: Option<Vec<InputMapping>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VwfdControl {
    pub id: String,
    pub control_type: Option<String>,
    pub split_behavior: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VwfdFlow {
    pub id: String,
    pub from: FlowEndpoint,
    pub to: FlowEndpoint,
    #[serde(rename = "type")]
    pub flow_type: Option<String>,
    pub condition: Option<String>,
    pub priority: Option<i8>,
    pub detached: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FlowEndpoint {
    pub node: String,
    pub port: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VwfdVariable {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub var_type: Option<String>,
}

// ── Type-Specific Configs ──

#[derive(Debug, Deserialize, Serialize)]
pub struct TriggerConfig {
    pub trigger_type: Option<String>,
    pub route: Option<String>,
    pub response_mode: Option<String>,
    pub stream_format: Option<String>,
    pub end_activity: Option<String>,
    pub filter: Option<String>,
    pub transform: Option<String>,
    pub webhook: Option<serde_json::Value>,
    pub cron: Option<CronConfig>,
    pub kafka: Option<serde_json::Value>,
    pub sftp: Option<serde_json::Value>,
    pub cdc: Option<serde_json::Value>,
    pub email: Option<serde_json::Value>,
    pub nats: Option<serde_json::Value>,
    pub grpc: Option<serde_json::Value>,
    pub s3_event: Option<serde_json::Value>,
    pub mqtt: Option<serde_json::Value>,
    pub webhook_config: Option<WebhookLegacyConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CronConfig {
    pub expression: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WebhookLegacyConfig {
    pub path: Option<String>,
    pub method: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConnectorConfig {
    pub connector_type: Option<String>,
    pub connector_ref: Option<String>,
    pub operation: Option<String>,
    pub streaming: Option<bool>,
    pub stream_format: Option<String>,
    pub timeout_ms: Option<u32>,
    pub retry_policy: Option<RetryPolicyConfig>,
    pub params: Option<serde_json::Value>,
    // HTTP-specific
    pub format: Option<String>,
    pub dialect: Option<String>,
    pub json_tap: Option<String>,
    pub done_marker: Option<String>,
    pub bearer_token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RetryPolicyConfig {
    pub max_attempts: Option<u32>,
    pub base_delay_ms: Option<u64>,
    pub max_delay_ms: Option<u64>,
    pub backoff_factor: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RuleConfig {
    pub rule_set_id: Option<String>,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EndTriggerConfig {
    pub trigger_ref: Option<String>,
    pub final_response: Option<FinalResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FinalResponse {
    pub language: Option<String>,
    pub source: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EndConfig {
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InputMapping {
    pub target: Option<String>,
    pub source: Option<MappingSource>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MappingSource {
    pub language: Option<String>,
    pub source: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoopConfig {
    pub condition: Option<String>,
    pub collection: Option<String>,
    pub item_variable: Option<String>,
    pub repeat_count: Option<u32>,
    pub max_iterations: Option<u32>,
}

impl TriggerConfig {
    /// Get webhook path from new spec `route` or legacy `webhook_config.path`.
    pub fn webhook_path(&self) -> Option<String> {
        self.route.clone()
            .or_else(|| self.webhook_config.as_ref().and_then(|w| w.path.clone()))
    }
}

// ── WASM Function config ────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct WasmConfig {
    pub module_ref: String,
    #[serde(default = "default_wasm_fn")]
    pub function_name: String,
    pub pool_size: Option<u32>,
    pub max_memory_pages: Option<u32>,
    pub timeout_ms: Option<u32>,
}
fn default_wasm_fn() -> String { "execute".into() }

// ── Sidecar config ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct SidecarActivityConfig {
    pub target: String,
    #[serde(default = "default_sidecar_method")]
    pub method: String,
    pub command: Option<String>,
    pub source: Option<String>,
    pub pool_size: Option<u32>,
    pub shm_size: Option<u64>,
    pub timeout_ms: Option<u32>,
    pub failover_target: Option<String>,
    pub fallback_wasm: Option<String>,
}
fn default_sidecar_method() -> String { "execute".into() }

// ── SubWorkflow config ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct SubWorkflowConfig {
    pub workflow_ref: String,
    pub timeout_ms: Option<u32>,
    pub input_strategy: Option<String>,
}

// ── HumanTask config ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct HumanTaskConfig {
    pub task_type: String,
    pub assignee: Option<String>,
    pub candidate_groups: Option<Vec<String>>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub form_ref: Option<String>,
    pub priority: Option<u8>,
    pub due_date: Option<String>,
    pub timeout_ms: Option<u64>,
    pub escalation_target: Option<String>,
}

// ── NativeCode config ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct NativeCodeConfig {
    pub handler_ref: String,
    pub timeout_ms: Option<u32>,
    pub exec_class: Option<String>,
}
