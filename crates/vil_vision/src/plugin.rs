//! VilPlugin implementation for image analysis integration.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::analyzer::{ImageAnalyzer, NoopAnalyzer};
use crate::config::VisionConfig;
use crate::handlers;
use crate::semantic::{VisionEvent, VisionFault, VisionState};

/// Vision plugin — image captioning and visual QA.
pub struct VisionPlugin {
    analyzer: Arc<dyn ImageAnalyzer>,
    config: Arc<VisionConfig>,
}

impl VisionPlugin {
    pub fn new(analyzer: Arc<dyn ImageAnalyzer>, config: VisionConfig) -> Self {
        Self {
            analyzer,
            config: Arc::new(config),
        }
    }
}

impl Default for VisionPlugin {
    fn default() -> Self {
        Self {
            analyzer: Arc::new(NoopAnalyzer),
            config: Arc::new(VisionConfig::default()),
        }
    }
}

impl VilPlugin for VisionPlugin {
    fn id(&self) -> &str {
        "vil-vision"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Image captioning, object detection, and visual QA"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "vision".into(),
            endpoints: vec![
                EndpointSpec::post("/api/vision/analyze").with_description("Analyze an image"),
                EndpointSpec::get("/api/vision/stats").with_description("Vision service stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn register(&self, ctx: &mut PluginContext) {
        let svc = ServiceProcess::new("vision")
            .state(handlers::VisionAnalyzer(Arc::clone(&self.analyzer)))
            .state(Arc::clone(&self.config))
            .endpoint(Method::POST, "/analyze", post(handlers::analyze_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<VisionEvent>()
            .faults::<VisionFault>()
            .manages::<VisionState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
