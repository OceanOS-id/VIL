// =============================================================================
// VIL Operator — VilServer CRD Definition
// =============================================================================
#![allow(dead_code)]

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// VilServer Custom Resource spec.
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "vil.dev",
    version = "v1alpha1",
    kind = "VilServer",
    namespaced,
    status = "VilServerStatus",
    shortname = "vs"
)]
#[serde(rename_all = "camelCase")]
pub struct VilServerSpec {
    /// Container image
    #[serde(default = "default_image")]
    pub image: String,
    /// Replica count
    #[serde(default = "default_replicas")]
    pub replicas: i32,
    /// HTTP port
    #[serde(default = "default_port")]
    pub port: i32,
    /// Metrics port
    #[serde(default = "default_metrics_port")]
    pub metrics_port: i32,
    /// SHM configuration
    #[serde(default)]
    pub shm: ShmSpec,
    /// Service definitions
    #[serde(default)]
    pub services: Vec<ServiceSpec>,
    /// Mesh routes
    #[serde(default)]
    pub mesh: MeshSpec,
    /// Resource requests/limits
    #[serde(default)]
    pub resources: Option<ResourceSpec>,
    /// Autoscaling
    #[serde(default)]
    pub autoscaling: Option<AutoscalingSpec>,
}

fn default_image() -> String { "ghcr.io/oceanos-id/vil-server:latest".into() }
fn default_replicas() -> i32 { 1 }
fn default_port() -> i32 { 8080 }
fn default_metrics_port() -> i32 { 9090 }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShmSpec {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_shm_size")]
    pub size_limit: String,
}

fn default_true() -> bool { true }
fn default_shm_size() -> String { "256Mi".into() }

impl Default for ShmSpec {
    fn default() -> Self {
        Self { enabled: true, size_limit: "256Mi".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServiceSpec {
    pub name: String,
    #[serde(default = "default_public")]
    pub visibility: String,
    #[serde(default)]
    pub prefix: Option<String>,
}

fn default_public() -> String { "public".into() }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct MeshSpec {
    #[serde(default = "default_unified")]
    pub mode: String,
    #[serde(default)]
    pub routes: Vec<RouteSpec>,
}

fn default_unified() -> String { "unified".into() }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RouteSpec {
    pub from: String,
    pub to: String,
    #[serde(default = "default_lane")]
    pub lane: String,
}

fn default_lane() -> String { "data".into() }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceSpec {
    pub requests: Option<ResourceValues>,
    pub limits: Option<ResourceValues>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceValues {
    pub cpu: Option<String>,
    pub memory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutoscalingSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_min")]
    pub min_replicas: i32,
    #[serde(default = "default_max")]
    pub max_replicas: i32,
    #[serde(default = "default_cpu_target")]
    pub target_cpu: i32,
}

fn default_min() -> i32 { 1 }
fn default_max() -> i32 { 10 }
fn default_cpu_target() -> i32 { 80 }

/// VilServer status (updated by operator).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct VilServerStatus {
    pub phase: String,
    pub replicas: i32,
    pub ready_replicas: i32,
    pub message: String,
}
