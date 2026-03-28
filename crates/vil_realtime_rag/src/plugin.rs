use vil_server::prelude::*;

use crate::config::RealtimeRagConfig;
use crate::handlers;
use crate::pipeline::RealtimeRagPipeline;
use crate::semantic::{RealtimeRagEvent, RealtimeRagFault, RealtimeRagState};
use std::sync::Arc;

pub struct RealtimeRagPlugin {
    config: RealtimeRagConfig,
}

impl RealtimeRagPlugin {
    pub fn new(config: RealtimeRagConfig) -> Self {
        Self { config }
    }
}

impl VilPlugin for RealtimeRagPlugin {
    fn id(&self) -> &str {
        "vil-realtime-rag"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "Sub-millisecond RAG pipeline for latency-critical applications"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "realtime-rag".into(),
            endpoints: vec![
                EndpointSpec::post("/api/realtime-rag/query"),
                EndpointSpec::get("/api/realtime-rag/stats"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let pipeline = Arc::new(RealtimeRagPipeline::new(self.config.clone()));
        ctx.provide::<Arc<RealtimeRagPipeline>>("realtime-rag", pipeline.clone());

        let svc = ServiceProcess::new("realtime-rag")
            .endpoint(Method::POST, "/query", post(handlers::query_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(pipeline)
            .emits::<RealtimeRagEvent>()
            .faults::<RealtimeRagFault>()
            .manages::<RealtimeRagState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
