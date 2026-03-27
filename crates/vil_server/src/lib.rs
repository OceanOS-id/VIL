// =============================================================================
// VIL Server — Process-Oriented Modular Server for Rust
// =============================================================================
//
// vil-server combines the developer experience of Axum with VIL's unique
// capabilities: zero-copy SHM IPC, Tri-Lane protocol, and automatic observability.
//
// # Quick Start
//
// ```no_run
// use vil_server::prelude::*;
//
// #[tokio::main]
// async fn main() {
//     vil_server::new("my-app")
//         .port(8080)
//         .route("/", get(|| async { "Hello from vil-server!" }))
//         .run()
//         .await;
// }
// ```

// Re-export all sub-crates
pub use vil_server_core as core;
// Hidden re-export so proc-macro generated code (vil_server_macros)
// can resolve types via ::vil_server::__private:: (all from vil_server_core
// to avoid axum version conflicts)
#[doc(hidden)]
pub mod __private {
    pub use vil_server_core::*;
    pub use vil_server_core::response;
    pub use vil_server_core::axum;
    pub use vil_server_core::tracing;
    pub use vil_log;
}
pub use vil_server_web as web;
pub use vil_server_config as config;
pub use vil_server_mesh as mesh;
pub use vil_server_auth as auth;
pub use vil_server_db as db;

// Re-export vil_sdk — required for semantic type macros (vil_state, vil_event, etc.)
// These macros generate code referencing ::vil_sdk::types::*, so the extern crate
// must be visible when users depend on vil_server instead of vil_sdk directly.
pub use vil_sdk;

// Re-export the main server builder
pub use vil_server_core::VilServer;
pub use vil_server_core::VilError;
pub use vil_server_core::AppState;

// Re-export Axum essentials
pub use axum;
pub use axum::extract::{Json, Path, Query, State};
pub use axum::response::IntoResponse;
pub use axum::routing::{delete, get, patch, post, put};
pub use axum::Router;
pub use tokio;
pub use tower;
pub use serde;
pub use tracing;

// Re-export mesh types
pub use vil_server_mesh::{Lane, MeshMode, MeshBuilder};

// Re-export web types
pub use vil_server_web::{Valid, HandlerError, HandlerResult};

// Re-export auth types
pub use vil_server_auth::{JwtAuth, RateLimit};

// Re-export config types
pub use vil_server_config::ServerConfig;

// Sprint 2: SHM extractors, process isolation, sync handler
pub use vil_server_core::ShmSlice;
pub use vil_server_core::ShmResponse;
pub use vil_server_core::ShmJson;
pub use vil_server_core::blocking_with;

// VIL macros — semantic types and handler macros
pub use vil_macros::{VilModel, VilError as DeriveVilError};
pub use vil_macros::{vil_state, vil_event, vil_fault, vil_decision};
pub use vil_server_macros::{vil_handler, VilSseEvent, VilWsEvent, vil_endpoint, vil_app, vil_service_state, vil_service};

// Tier B AI Semantic macros
pub use vil_macros::{VilAiEvent, VilAiFault, VilAiState, VilAiDecision};

// VIL JSON — high-performance JSON abstraction
pub use vil_json;

// WebSocket hub
pub use vil_server_core::WsHub;
pub use vil_server_core::SseHub;
pub use vil_server_core::{SseEvent, sse_stream, sse_stream_with_keepalive};

// VX — Process-Oriented Server (Tri-Lane architecture)
pub use vil_server_core::VilApp;
pub use vil_server_core::ServiceProcess;
pub use vil_server_core::ServiceCtx;
pub use vil_server_core::VxMeshConfig;
pub use vil_server_core::VxFailoverConfig;
pub use vil_server_core::FailoverStrategy;
pub use vil_server_core::ExecClass;
pub use vil_server_core::VxLane;
pub use vil_server_core::RequestDescriptor;
pub use vil_server_core::ResponseDescriptor;

// Plugin System
pub use vil_server_core::{
    VilPlugin, PluginCapability, PluginDependency, PluginHealth,
    PluginContext, ResourceRegistry,
};

/// Convenience constructor for VilServer.
pub fn new(name: impl Into<String>) -> VilServer {
    VilServer::new(name)
}

/// Prelude module — import everything you need with `use vil_server::prelude::*`.
pub mod prelude {
    // Server builder
    pub use crate::VilServer;
    pub use crate::new;

    // Axum essentials
    pub use axum::extract::{Json, Path, Query, State};
    pub use axum::response::IntoResponse;
    pub use axum::routing::{delete, get, patch, post, put};
    pub use axum::Router;
    pub use axum::http::StatusCode;

    // VIL types
    pub use crate::VilError;
    pub use crate::AppState;
    pub use vil_server_web::{Valid, HandlerResult};
    pub use vil_server_core::RequestId;
    pub use vil_server_core::response::{VilResponse, NoContent};
    pub use vil_server_core::router::{ServiceDef, Visibility};

    // Mesh types
    pub use vil_server_mesh::{Lane, MeshMode, MeshBuilder};

    // Auth types
    pub use vil_server_auth::{JwtAuth, RateLimit};

    // SHM zero-copy (Sprint 2)
    pub use vil_server_core::ShmSlice;
    pub use vil_server_core::shm_response::{ShmResponse, ShmJson};
    pub use vil_server_core::sync_handler::blocking_with;
    pub use vil_server_core::ShmContext;

    // Serde
    pub use serde::{Deserialize, Serialize};

    // VIL macros — derive macros for semantic types
    pub use vil_macros::{VilModel, VilError as DeriveVilError};
    pub use vil_macros::{vil_state, vil_event, vil_fault, vil_decision};
    pub use vil_server_macros::{vil_handler, VilSseEvent, VilWsEvent, vil_endpoint, vil_app, vil_service_state, vil_service};

    // Tier B AI Semantic
    pub use vil_macros::{VilAiEvent, VilAiFault, VilAiState, VilAiDecision};
    pub use vil_server_core::plugin_system::semantic::{AiSemantic, AiSemanticKind, AiLane, AiSemanticEnvelope};

    // VIL model trait
    pub use vil_server_core::model::VilModel as VilModelTrait;

    // VX — Process-Oriented Server (Tri-Lane architecture)
    pub use vil_server_core::VilApp;
    pub use vil_server_core::ServiceProcess;
    pub use vil_server_core::ServiceCtx;
    pub use vil_server_core::VxMeshConfig;
    pub use vil_server_core::VxFailoverConfig;
    pub use vil_server_core::FailoverStrategy;
    pub use vil_server_core::ExecClass;
    pub use vil_server_core::VxLane;

    // SSE + WebSocket streaming
    pub use vil_server_core::{SseHub, SseEvent, sse_stream, sse_stream_with_keepalive};
    pub use vil_server_core::WsHub;

    // Axum Method for VX endpoint registration
    pub use axum::http::Method;

    // Axum Extension extractor (commonly used for shared state injection)
    pub use axum::extract::Extension;

    // Common std types
    pub use std::sync::Arc;

    // SSE Collector with built-in async client (upstream proxy without boilerplate)
    pub use vil_server_core::{SseCollect, SseCollectError, SseDialect};

    // Plugin System
    pub use vil_server_core::{
        VilPlugin, PluginCapability, PluginDependency, PluginHealth,
        PluginContext, ResourceRegistry,
    };
    pub use vil_server_core::plugin_system::EndpointSpec;

}
