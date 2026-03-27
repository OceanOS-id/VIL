//! VilPlugin implementation for Prompt Shield integration.
//!
//! Registers a ServiceProcess with `/scan` and `/stats` endpoints,
//! provides `Arc<PromptShield>` as a shared resource for other plugins.

use std::sync::{Arc, Mutex};

use axum::http::Method;
use axum::routing::{get, post};

use vil_server_core::plugin_system::{
    EndpointSpec, PluginCapability, PluginContext, PluginHealth, VilPlugin,
};
use vil_server_core::vx::service::ServiceProcess;

use crate::config::ShieldConfig;
use crate::detector::PromptShield;
use crate::handlers::{self, ShieldServiceState};
use crate::semantic::{ShieldEvent, ShieldFault, ShieldState};

/// Prompt Shield plugin — real-time prompt injection detection.
///
/// # Example
/// ```ignore
/// VilApp::new("ai-gateway")
///     .plugin(ShieldPlugin::new())
///     .run().await;
/// ```
pub struct ShieldPlugin {
    config: ShieldConfig,
}

impl ShieldPlugin {
    pub fn new() -> Self {
        Self {
            config: ShieldConfig::default(),
        }
    }

    pub fn with_config(config: ShieldConfig) -> Self {
        Self { config }
    }
}

impl Default for ShieldPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for ShieldPlugin {
    fn id(&self) -> &str {
        "vil-prompt-shield"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Real-time prompt injection detection (<100us latency)"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![
            PluginCapability::Resource {
                type_name: "PromptShield",
                name: "shield".into(),
            },
            PluginCapability::Service {
                name: "shield".into(),
                endpoints: vec![
                    EndpointSpec::post("/api/shield/scan")
                        .with_description("Scan text for prompt injection"),
                    EndpointSpec::get("/api/shield/stats")
                        .with_description("Shield scan statistics"),
                ],
            },
        ]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let shield = Arc::new(PromptShield::with_config(self.config.clone()));
        let state = Arc::new(Mutex::new(ShieldState::default()));

        ctx.provide::<Arc<PromptShield>>("shield", shield.clone());

        let svc = ServiceProcess::new("shield")
            .prefix("/api/shield")
            .endpoint(Method::POST, "/scan", post(handlers::scan_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(ShieldServiceState { shield, state })
            .emits::<ShieldEvent>()
            .faults::<ShieldFault>()
            .manages::<ShieldState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
