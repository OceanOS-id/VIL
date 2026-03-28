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
pub mod server;
pub mod router;
pub mod health;
pub mod state;
pub mod error;
pub mod shutdown;
pub mod model;
pub mod process;

// ─── HTTP Layer ─────────────────────────────────────────────────────────────
pub mod extractors;
pub mod response;
pub mod sync_handler;
pub mod sse;
pub mod sse_collect;
pub mod websocket;
pub mod grpc;
pub mod profiler;
pub mod http_client;
pub mod content_negotiation;
pub mod etag;

// ─── SHM Bridge ─────────────────────────────────────────────────────────────
pub mod shm_extractor;
pub mod shm_response;
pub mod shm_pool;
pub mod shm_query_cache;

// ─── Middleware ──────────────────────────────────────────────────────────────
pub mod middleware;
pub mod middleware_stack;
pub mod middleware_dsl;
pub mod obs_middleware;
pub mod timeout;
pub mod compression;
pub mod request_log;
pub mod idempotency;
pub mod tls;
pub mod retry;
pub mod coalescing;
pub mod multi_protocol;

// ─── Observability ──────────────────────────────────────────────────────────
pub mod otel;
pub mod trace_middleware;
pub mod custom_metrics;
pub mod diagnostics;
pub mod error_tracker;
pub mod alerting;
pub mod upstream_metrics;

// ─── WASM / Capsule ─────────────────────────────────────────────────────────
pub mod wasm_host;
pub mod wasm_dispatch;
pub mod wasm_shm_bridge;
pub mod capsule_handler;

// ─── Plugin System ──────────────────────────────────────────────────────────
pub mod plugin;
pub mod plugin_manifest;
pub mod plugin_manager;
pub mod plugin_api;
pub mod plugin_detail_gui;
pub mod plugin_system;

// ─── Production Infrastructure ──────────────────────────────────────────────
pub mod cache;
pub mod scheduler;
pub mod feature_flags;
pub mod streaming;
pub mod api_versioning;
pub mod rolling_restart;
pub mod hot_reload;
pub mod playground;
pub mod secrets;
pub mod sidecar_admin;

// ─── VX: Process-Oriented Server Architecture (Tri-Lane) ────────────────────
pub mod vx;

// =============================================================================
// Re-exports for convenience
// =============================================================================

pub use server::VilServer;
pub use error::VilError;
pub use model::VilModel;
pub use state::AppState;
pub use extractors::RequestId;

// Re-export Axum essentials so users don't need to depend on axum directly
pub use axum;
pub use axum::extract::{Json, Path, Query, State};
pub use axum::response::IntoResponse;
pub use axum::routing::{delete, get, patch, post, put};
pub use axum::Router;
pub use tower;
pub use tower_http;
pub use tracing;
pub use axum::http::StatusCode;

// Re-export tokio for the runtime
pub use tokio;

// Re-export VIL runtime types for handlers
pub use vil_rt::VastarRuntimeWorld;
pub use vil_shm::ExchangeHeap;
pub use extractors::ShmContext;

// SHM bridge exports
pub use shm_extractor::ShmSlice;
pub use shm_response::{ShmResponse, ShmJson};
pub use process::ProcessRegistry;
pub use obs_middleware::HandlerMetricsRegistry;
pub use sync_handler::{blocking, blocking_with};

// VX re-exports
pub use vx::app::{VilApp, VxMeshConfig, VxFailoverConfig, FailoverStrategy};
pub use vx::service::ServiceProcess;
pub use vx::ctx::{ServiceCtx, ServiceName};
pub use vx::descriptor::{RequestDescriptor, ResponseDescriptor};
pub use vx::endpoint::ExecClass;
pub use vx::tri_lane::Lane as VxLane;
pub use vx::ingress::IngressBridge;
pub use vx::egress::EgressHandle;
pub use vx::kernel::{VxKernel, TokenState, ControlSignal, KernelMetrics, MetricsSnapshot};
pub use vx::cleanup::{CleanupConfig, CleanupReport, spawn_cleanup_task};

// WebSocket hub re-export
pub use streaming::{WsHub, SseHub};
pub use sse::{SseEvent, sse_stream, sse_stream_with_keepalive};

// Sidecar re-exports
pub use vil_sidecar::{SidecarConfig, SidecarRegistry, SidecarHealth};

// SSE Collector for VilApp handlers
pub use sse_collect::{SseCollect, SseCollectError, SseDialect};
pub use reqwest;

// Plugin System re-exports
pub use plugin_system::{
    VilPlugin, PluginCapability, EndpointSpec as PluginEndpointSpec,
    PluginDependency, PluginHealth, PluginRegistry, PluginError, PluginInfo,
    ResourceRegistry, PluginContext,
};

// Tier B AI Semantic re-exports
pub use plugin_system::semantic::{AiSemantic, AiSemanticKind, AiLane, AiSemanticEnvelope};
