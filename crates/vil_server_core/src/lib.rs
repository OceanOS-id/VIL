// =============================================================================
// VIL Server Core — Process-Oriented Modular Server
// =============================================================================
//
// Built on Axum + Tower + Tokio, layered with VIL zero-copy SHM,
// Tri-Lane protocol, and automatic observability.
//
// Module Organization:
//   core/       — server builder, router, state, error, health, shutdown
//   http/       — extractors, response, request handling, SSE, WebSocket
//   shm/        — shared memory extractors, response, pool, query cache
//   mw/         — middleware stack (timeout, compression, CORS, TLS, etc.)
//   observe/    — observability (OTel, tracing, metrics, diagnostics)
//   wasm/       — WASM host, dispatch, SHM bridge, capsule handler
//   plugins/    — plugin system, manifest, manager, API, GUI
//   production/ — cache, scheduler, feature flags, rolling restart, versioning
//   vx/         — process-oriented server architecture (Tri-Lane)

// ─── Core ───────────────────────────────────────────────────────────────────
pub mod error;
pub mod health;
pub mod model;
pub mod process;
pub mod router;
pub mod server;
pub mod shutdown;
pub mod state;

// ─── HTTP Layer ─────────────────────────────────────────────────────────────
pub mod content_negotiation;
pub mod etag;
pub mod extractors;
pub mod grpc;
pub mod http_client;
pub mod profiler;
pub mod response;
pub mod sse;
pub mod sse_collect;
pub mod sync_handler;
pub mod websocket;

// ─── SHM Bridge ─────────────────────────────────────────────────────────────
pub mod shm_extractor;
pub mod shm_pool;
pub mod shm_query_cache;
pub mod shm_response;

// ─── Middleware ──────────────────────────────────────────────────────────────
pub mod coalescing;
pub mod compression;
pub mod idempotency;
pub mod middleware;
pub mod middleware_dsl;
pub mod middleware_stack;
pub mod multi_protocol;
pub mod obs_middleware;
pub mod request_log;
pub mod retry;
pub mod timeout;
pub mod tls;

// ─── Observability ──────────────────────────────────────────────────────────
pub mod alerting;
pub mod custom_metrics;
pub mod diagnostics;
pub mod error_tracker;
pub mod otel;
pub mod trace_middleware;
pub mod upstream_metrics;

// ─── WASM / Capsule ─────────────────────────────────────────────────────────
pub mod capsule_handler;
pub mod wasm_dispatch;
pub mod wasm_host;
pub mod wasm_shm_bridge;

// ─── Plugin System ──────────────────────────────────────────────────────────
pub mod plugin;
pub mod plugin_api;
pub mod plugin_detail_gui;
pub mod plugin_manager;
pub mod plugin_manifest;
pub mod plugin_system;

// ─── Production Infrastructure ──────────────────────────────────────────────
pub mod api_versioning;
pub mod cache;
pub mod feature_flags;
pub mod hot_reload;
pub mod playground;
pub mod rolling_restart;
pub mod scheduler;
pub mod secrets;
pub mod sidecar_admin;
pub mod streaming;

// ─── VX: Process-Oriented Server Architecture (Tri-Lane) ────────────────────
pub mod vx;

// =============================================================================
// Re-exports for convenience
// =============================================================================

pub use error::VilError;
pub use extractors::RequestId;
pub use model::VilModel;
pub use server::VilServer;
pub use state::AppState;

// Re-export Axum essentials so users don't need to depend on axum directly
pub use axum;
pub use axum::extract::{Json, Path, Query, State};
pub use axum::http::StatusCode;
pub use axum::response::IntoResponse;
pub use axum::routing::{delete, get, patch, post, put};
pub use axum::Router;
pub use tower;
pub use tower_http;
pub use tracing;

// Re-export tokio for the runtime
pub use tokio;

// Re-export VIL runtime types for handlers
pub use extractors::ShmContext;
pub use vil_rt::VastarRuntimeWorld;
pub use vil_shm::ExchangeHeap;

// SHM bridge exports
pub use obs_middleware::HandlerMetricsRegistry;
pub use process::ProcessRegistry;
pub use shm_extractor::ShmSlice;
pub use shm_response::{ShmJson, ShmResponse};
pub use sync_handler::{blocking, blocking_with};

// VX re-exports
pub use vx::app::{FailoverStrategy, VilApp, VxFailoverConfig, VxMeshConfig};
pub use vx::cleanup::{spawn_cleanup_task, CleanupConfig, CleanupReport};
pub use vx::ctx::{ServiceCtx, ServiceName};
pub use vx::descriptor::{RequestDescriptor, ResponseDescriptor};
pub use vx::egress::EgressHandle;
pub use vx::endpoint::ExecClass;
pub use vx::ingress::IngressBridge;
pub use vx::kernel::{ControlSignal, KernelMetrics, MetricsSnapshot, TokenState, VxKernel};
pub use vx::service::ServiceProcess;
pub use vx::tri_lane::Lane as VxLane;

// WebSocket hub re-export
pub use sse::{sse_stream, sse_stream_with_keepalive, SseEvent};
pub use streaming::{SseHub, WsHub};

// Sidecar re-exports
pub use vil_sidecar::{SidecarConfig, SidecarHealth, SidecarRegistry};

// SSE Collector for VilApp handlers
pub use reqwest;
pub use sse_collect::{SseCollect, SseCollectError, SseDialect};

// Plugin System re-exports
pub use plugin_system::{
    EndpointSpec as PluginEndpointSpec, PluginCapability, PluginContext, PluginDependency,
    PluginError, PluginHealth, PluginInfo, PluginRegistry, ResourceRegistry, VilPlugin,
};

// Tier B AI Semantic re-exports
pub use plugin_system::semantic::{AiLane, AiSemantic, AiSemanticEnvelope, AiSemanticKind};
