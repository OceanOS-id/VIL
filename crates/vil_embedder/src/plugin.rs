use vil_server::prelude::*;

use std::sync::Arc;
use crate::provider::EmbedProvider;
use crate::openai::OpenAiEmbedder;
use crate::handlers;
use crate::semantic::{EmbedEvent, EmbedFault, EmbedderState};

pub struct EmbedderPlugin {
    api_key: String,
    model: String,
}

impl EmbedderPlugin {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self { api_key: api_key.into(), model: "text-embedding-3-small".into() }
    }
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into(); self
    }
}

impl VilPlugin for EmbedderPlugin {
    fn id(&self) -> &str { "vil-embedder" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Multi-model text embedding with batching" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "embedder".into(),
            endpoints: vec![
                EndpointSpec::post("/api/embedder/embed"),
                EndpointSpec::post("/api/embedder/similarity"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let provider: Arc<dyn EmbedProvider> = Arc::new(OpenAiEmbedder::new(&self.api_key));
        ctx.provide::<Arc<dyn EmbedProvider>>("embedder-provider", provider.clone());

        let svc = ServiceProcess::new("embedder")
            .endpoint(Method::POST, "/embed", post(handlers::embed_handler))
            .endpoint(Method::POST, "/similarity", post(handlers::similarity_handler))
            .state(provider)
            .emits::<EmbedEvent>()
            .faults::<EmbedFault>()
            .manages::<EmbedderState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
