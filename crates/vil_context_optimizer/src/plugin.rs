//! VilPlugin implementation for Context Optimizer integration.
//!
//! Registers a ServiceProcess with `/optimize` and `/stats` endpoints.

use std::sync::{Arc, Mutex};

use axum::http::Method;
use axum::routing::{get, post};

use vil_server_core::plugin_system::{
    EndpointSpec, PluginCapability, PluginContext, PluginHealth, VilPlugin,
};
use vil_server_core::vx::service::ServiceProcess;

use crate::handlers;
use crate::semantic::{OptimizeEvent, OptimizeFault, OptimizerState};

/// Context Optimizer plugin — intelligent context compression for LLMs.
///
/// # Example
/// ```ignore
/// VilApp::new("ai-service")
///     .plugin(OptimizerPlugin::new())
///     .run().await;
/// ```
pub struct OptimizerPlugin;

impl OptimizerPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OptimizerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for OptimizerPlugin {
    fn id(&self) -> &str {
        "vil-context-optimizer"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Intelligent context compression — fit 4-10x more context in LLM token budgets"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "optimizer".into(),
            endpoints: vec![
                EndpointSpec::post("/api/optimizer/optimize")
                    .with_description("Optimize context chunks to fit token budget"),
                EndpointSpec::get("/api/optimizer/stats")
                    .with_description("Optimizer statistics"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let state = Arc::new(Mutex::new(OptimizerState::default()));

        let svc = ServiceProcess::new("optimizer")
            .prefix("/api/optimizer")
            .endpoint(Method::POST, "/optimize", post(handlers::optimize_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(state)
            .emits::<OptimizeEvent>()
            .faults::<OptimizeFault>()
            .manages::<OptimizerState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
