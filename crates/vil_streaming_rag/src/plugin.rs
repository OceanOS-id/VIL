//! VilPlugin implementation for Streaming RAG.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::semantic::{StreamingRagEvent, StreamingRagFault, StreamingRagState};
use crate::stream::StreamingIngester;

pub struct StreamingRagPlugin {
    ingester: Arc<StreamingIngester>,
}

impl StreamingRagPlugin {
    pub fn new(ingester: Arc<StreamingIngester>) -> Self {
        Self { ingester }
    }
}

impl VilPlugin for StreamingRagPlugin {
    fn id(&self) -> &str { "vil-streaming-rag" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Chunk-by-chunk streaming retrieval-augmented generation" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "streaming-rag".into(),
            endpoints: vec![
                EndpointSpec::post("/api/streaming-rag/query"),
                EndpointSpec::get("/api/streaming-rag/stats"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        ctx.provide::<Arc<StreamingIngester>>("streaming-rag", self.ingester.clone());

        let svc = ServiceProcess::new("streaming-rag")
            .state(self.ingester.clone())
            .emits::<StreamingRagEvent>()
            .faults::<StreamingRagFault>()
            .manages::<StreamingRagState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
