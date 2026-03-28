use std::sync::Arc;
use vil_server::prelude::*;

use crate::handlers;
use crate::pipeline::DataPipeline;
use crate::vil_semantic::{DataPrepEvent, DataPrepFault, DataPrepState};

pub struct DataPrepPlugin {
    pipeline: Arc<DataPipeline>,
}

impl DataPrepPlugin {
    pub fn new() -> Self {
        Self {
            pipeline: Arc::new(DataPipeline::new()),
        }
    }

    /// Create with a pre-configured pipeline.
    pub fn with_pipeline(pipeline: Arc<DataPipeline>) -> Self {
        Self { pipeline }
    }

    /// Access the shared pipeline.
    pub fn pipeline(&self) -> &Arc<DataPipeline> {
        &self.pipeline
    }
}

impl Default for DataPrepPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for DataPrepPlugin {
    fn id(&self) -> &str {
        "vil-data-prep"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Dataset preparation and cleaning for LLM fine-tuning"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "data-prep".into(),
            endpoints: vec![EndpointSpec::get("/api/data-prep/stats")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let pipeline = Arc::clone(&self.pipeline);

        let svc = ServiceProcess::new("data-prep")
            .state(pipeline)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<DataPrepEvent>()
            .faults::<DataPrepFault>()
            .manages::<DataPrepState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
