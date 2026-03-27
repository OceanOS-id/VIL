// =============================================================================
// VIL Plugin SDK — Stable Community Plugin Interface
// =============================================================================
//
// This crate is the ONLY dependency community plugin authors need.
// It re-exports the stable plugin API surface from vil_server_core and adds
// ergonomic utilities for building, testing, and declaring plugins.
//
// Stability guarantee: public API in this crate follows semver.
// Internal vil_server_core changes will NOT break plugin authors.
//
// Quick start:
//   use vil_plugin_sdk::prelude::*;
//
//   pub struct MyPlugin;
//   impl VilPlugin for MyPlugin {
//       fn id(&self) -> &str { "my-plugin" }
//       fn version(&self) -> &str { "1.0.0" }
//       fn register(&self, ctx: &mut PluginContext) { ... }
//   }

pub mod prelude;
pub mod builder;
pub mod manifest;
pub mod testing;

// ── Stable re-exports from vil_server_core ──────────────────────────────

// Core trait + plugin system types
pub use vil_server_core::{
    VilPlugin,
    PluginContext,
    ResourceRegistry,
    PluginRegistry,
    PluginCapability,
    PluginEndpointSpec as EndpointSpec,
    PluginDependency,
    PluginHealth,
    PluginInfo,
    PluginError,
};

// Service building
pub use vil_server_core::ServiceProcess;
pub use vil_server_core::VxLane;

// Handler types
pub use vil_server_core::error::VilError;
pub use vil_server_core::response::VilResponse;
pub use vil_server_core::ServiceCtx;
pub use vil_server_core::ShmSlice;

// Axum routing (plugins need these for endpoint registration)
pub use vil_server_core::axum::routing::{get, post, put, delete};
pub use vil_server_core::axum::http::Method;

// Re-export serde for plugin config types
pub use serde;
pub use serde_json;
