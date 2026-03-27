//! VilPlugin implementation for Quantized Runtime integration.
//!
//! Registers a ServiceProcess with `/infer` and `/stats` endpoints.

use std::sync::{Arc, Mutex};

use axum::http::Method;
use axum::routing::{get, post};

use vil_server_core::plugin_system::{
    EndpointSpec, PluginCapability, PluginContext, PluginHealth, VilPlugin,
};
use vil_server_core::vx::service::ServiceProcess;

use crate::config::QuantizedModelConfig;
use crate::runtime::QuantizedRuntime;
use crate::handlers::{self, QuantizedServiceState};
use crate::semantic::{QuantizeEvent, QuantizeFault, QuantizedState};

/// Quantized Runtime plugin — model quantization and inference.
///
/// # Example
/// ```ignore
/// VilApp::new("ai-service")
///     .plugin(QuantizedPlugin::new(config))
///     .run().await;
/// ```
pub struct QuantizedPlugin {
    config: QuantizedModelConfig,
    auto_load: bool,
}

impl QuantizedPlugin {
    pub fn new(config: QuantizedModelConfig) -> Self {
        Self {
            config,
            auto_load: false,
        }
    }

    /// Auto-load the model on plugin registration.
    pub fn auto_load(mut self) -> Self {
        self.auto_load = true;
        self
    }
}

impl VilPlugin for QuantizedPlugin {
    fn id(&self) -> &str {
        "vil-quantized"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Model quantization runtime — GGUF/GGML inference"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![
            PluginCapability::Resource {
                type_name: "QuantizedRuntime",
                name: "quantized".into(),
            },
            PluginCapability::Service {
                name: "quantized".into(),
                endpoints: vec![
                    EndpointSpec::post("/api/quantized/infer")
                        .with_description("Run inference on quantized model"),
                    EndpointSpec::get("/api/quantized/stats")
                        .with_description("Quantized runtime statistics"),
                ],
            },
        ]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let mut runtime = QuantizedRuntime::new(self.config.clone());

        if self.auto_load {
            if let Err(e) = runtime.load() {
                tracing::warn!("QuantizedPlugin: auto-load failed: {}", e);
            }
        }

        let runtime = Arc::new(Mutex::new(runtime));
        let state = Arc::new(Mutex::new(QuantizedState::default()));

        let svc_state = QuantizedServiceState {
            runtime,
            state,
        };

        let svc = ServiceProcess::new("quantized")
            .prefix("/api/quantized")
            .endpoint(Method::POST, "/infer", post(handlers::infer_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(svc_state)
            .emits::<QuantizeEvent>()
            .faults::<QuantizeFault>()
            .manages::<QuantizedState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
