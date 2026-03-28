//! VilPlugin implementation for document extraction integration.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::extractor::DataExtractor;
use crate::handlers;
use crate::rules::RuleExtractor;
use crate::semantic::{ExtractEvent, ExtractFault, ExtractState};

/// Document extraction plugin — structured data extraction from text.
pub struct DocExtractPlugin {
    extractor: Arc<dyn DataExtractor>,
}

impl DocExtractPlugin {
    pub fn new(extractor: Arc<dyn DataExtractor>) -> Self {
        Self { extractor }
    }
}

impl Default for DocExtractPlugin {
    fn default() -> Self {
        Self {
            extractor: Arc::new(RuleExtractor::new()),
        }
    }
}

impl VilPlugin for DocExtractPlugin {
    fn id(&self) -> &str {
        "vil-doc-extract"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Structured data extraction from documents using rule-based patterns"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "extract".into(),
            endpoints: vec![
                EndpointSpec::post("/api/extract/extract")
                    .with_description("Extract structured data"),
                EndpointSpec::get("/api/extract/stats").with_description("Extraction stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn register(&self, ctx: &mut PluginContext) {
        let svc = ServiceProcess::new("doc-extract")
            .state(Arc::clone(&self.extractor))
            .endpoint(Method::POST, "/extract", post(handlers::extract_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<ExtractEvent>()
            .faults::<ExtractFault>()
            .manages::<ExtractState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
