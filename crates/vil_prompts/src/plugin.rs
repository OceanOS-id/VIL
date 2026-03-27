use vil_server::prelude::*;

use std::sync::{Arc, RwLock};
use crate::{PromptRegistry, rag_qa_template, summarize_template, code_review_template};
use crate::handlers;
use crate::semantic::{PromptRenderEvent, PromptFault, PromptsState};

pub struct PromptsPlugin;

impl PromptsPlugin {
    pub fn new() -> Self { Self }
}

impl Default for PromptsPlugin {
    fn default() -> Self { Self::new() }
}

impl VilPlugin for PromptsPlugin {
    fn id(&self) -> &str { "vil-prompts" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "Prompt template engine with registry and rendering" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "prompts".into(),
            endpoints: vec![
                EndpointSpec::post("/api/prompts/render"),
                EndpointSpec::get("/api/prompts/list"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }

    fn register(&self, ctx: &mut PluginContext) {
        let mut registry = PromptRegistry::new();
        registry.register("rag_qa", rag_qa_template());
        registry.register("summarize", summarize_template());
        registry.register("code_review", code_review_template());
        let registry = Arc::new(RwLock::new(registry));

        ctx.provide::<Arc<RwLock<PromptRegistry>>>("prompts-registry", registry.clone());

        let svc = ServiceProcess::new("prompts")
            .endpoint(Method::POST, "/render", post(handlers::render_handler))
            .endpoint(Method::GET, "/list", get(handlers::list_handler))
            .state(registry)
            .emits::<PromptRenderEvent>()
            .faults::<PromptFault>()
            .manages::<PromptsState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
