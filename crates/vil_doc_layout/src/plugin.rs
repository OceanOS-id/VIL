use vil_server::prelude::*;
use std::sync::Arc;

use crate::analyzer::LayoutAnalyzer;
use crate::handlers;
use crate::semantic::{LayoutAnalyzeEvent, LayoutFault, DocLayoutState};

pub struct DocLayoutPlugin {
    analyzer: Arc<LayoutAnalyzer>,
}

impl DocLayoutPlugin {
    pub fn new() -> Self {
        Self {
            analyzer: Arc::new(LayoutAnalyzer::new()),
        }
    }

    /// Create with a pre-configured analyzer.
    pub fn with_analyzer(analyzer: Arc<LayoutAnalyzer>) -> Self {
        Self { analyzer }
    }

    /// Access the shared analyzer.
    pub fn analyzer(&self) -> &Arc<LayoutAnalyzer> {
        &self.analyzer
    }
}

impl Default for DocLayoutPlugin {
    fn default() -> Self { Self::new() }
}

impl VilPlugin for DocLayoutPlugin {
    fn id(&self) -> &str { "vil-doc-layout" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "Rule-based document layout analysis" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "doc-layout".into(),
            endpoints: vec![
                EndpointSpec::post("/api/layout/analyze"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }

    fn register(&self, ctx: &mut PluginContext) {
        let analyzer = Arc::clone(&self.analyzer);

        let svc = ServiceProcess::new("doc-layout")
            .state(analyzer)
            .endpoint(Method::POST, "/analyze", post(handlers::analyze_handler))
            .emits::<LayoutAnalyzeEvent>()
            .faults::<LayoutFault>()
            .manages::<DocLayoutState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
