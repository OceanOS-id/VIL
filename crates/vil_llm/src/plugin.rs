//! VilPlugin implementation for LLM integration.
//!
//! Registers a ServiceProcess with `/chat`, `/embed`, `/models` endpoints,
//! provides `Arc<dyn LlmProvider>` and `Arc<dyn EmbeddingProvider>` as shared
//! resources for other plugins (e.g., vil_rag, vil_agent).

use vil_server::prelude::*;

use std::sync::Arc;



use crate::anthropic::*;
use crate::extractors::{Embedder, Llm};
use crate::handlers::{self, LlmServiceState, ModelsResponseBody};
use crate::ollama::*;
use crate::openai::*;
use crate::provider::*;
use crate::router::*;
use crate::semantic::{LlmFault, LlmResponseEvent, LlmUsageState};

/// LLM plugin — multi-provider chat, embedding, and model routing.
///
/// # Example
/// ```ignore
/// VilApp::new("ai-service")
///     .plugin(
///         LlmPlugin::new()
///             .openai(OpenAiConfig::from_env("gpt-4o"))
///             .ollama(OllamaConfig::new("llama3"))
///             .strategy(RouterStrategy::Fallback)
///             .embedder_openai("sk-...", "text-embedding-3-small")
///     )
///     .run().await;
/// ```
pub struct LlmPlugin {
    providers: Vec<(String, Arc<dyn LlmProvider>)>,
    embedder: Option<Arc<dyn EmbeddingProvider>>,
    router_strategy: RouterStrategy,
}

impl LlmPlugin {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            embedder: None,
            router_strategy: RouterStrategy::Primary,
        }
    }

    pub fn openai(mut self, config: OpenAiConfig) -> Self {
        let name = format!("openai:{}", config.model);
        self.providers
            .push((name, Arc::new(OpenAiProvider::new(config))));
        self
    }

    pub fn anthropic(mut self, config: AnthropicConfig) -> Self {
        let name = format!("anthropic:{}", config.model);
        self.providers
            .push((name, Arc::new(AnthropicProvider::new(config))));
        self
    }

    pub fn ollama(mut self, config: OllamaConfig) -> Self {
        let name = format!("ollama:{}", config.model);
        self.providers
            .push((name, Arc::new(OllamaProvider::new(config))));
        self
    }

    pub fn embedder_openai(
        mut self,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        self.embedder = Some(Arc::new(OpenAiEmbedder::new(api_key, model)));
        self
    }

    pub fn strategy(mut self, strategy: RouterStrategy) -> Self {
        self.router_strategy = strategy;
        self
    }

    fn build_provider(&self) -> Option<Arc<dyn LlmProvider>> {
        if self.providers.len() > 1 {
            let mut router = LlmRouter::new(self.router_strategy);
            for (_, provider) in &self.providers {
                router = router.add_provider(provider.clone());
            }
            Some(Arc::new(router))
        } else {
            self.providers.first().map(|(_, p)| p.clone())
        }
    }
}

impl Default for LlmPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for LlmPlugin {
    fn id(&self) -> &str {
        "vil-llm"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Multi-provider LLM abstraction (chat, streaming, embeddings)"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![
            PluginCapability::Resource {
                type_name: "LlmProvider",
                name: "llm".into(),
            },
            PluginCapability::Service {
                name: "llm".into(),
                endpoints: vec![
                    EndpointSpec::post("/api/llm/chat").with_description("Chat completion"),
                    EndpointSpec::post("/api/llm/embed").with_description("Text embedding"),
                    EndpointSpec::get("/api/llm/models")
                        .with_description("List available models"),
                ],
            },
        ]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let llm = match self.build_provider() {
            Some(p) => p,
            None => {
                tracing::warn!("LlmPlugin: no providers configured");
                return;
            }
        };

        // Provide LLM resource for other plugins (vil_rag, vil_agent)
        ctx.provide::<Arc<dyn LlmProvider>>("llm", llm.clone());

        // Provide embedder if configured
        if let Some(ref embedder) = self.embedder {
            ctx.provide::<Arc<dyn EmbeddingProvider>>("embedder", embedder.clone());
        }

        // Build model list for /models endpoint
        let models = ModelsResponseBody {
            models: self.providers.iter().map(|(n, _)| n.clone()).collect(),
        };

        // Build combined service state
        let embedder_ext = self.embedder.as_ref().map(|e| Embedder::from(e.clone()));

        let llm_state = LlmServiceState {
            llm: Llm::from(llm.clone()),
            embedder: embedder_ext,
            models,
        };

        // Build ServiceProcess with VIL handler pattern
        let mut svc = ServiceProcess::new("llm")
            .endpoint(Method::POST, "/chat", post(handlers::chat_handler))
            .endpoint(Method::GET, "/models", get(handlers::models_handler))
            .state(llm_state)
            .emits::<LlmResponseEvent>()
            .faults::<LlmFault>()
            .manages::<LlmUsageState>();

        // Only add /embed if embedder is configured
        if self.embedder.is_some() {
            svc = svc
                .endpoint(Method::POST, "/embed", post(handlers::embed_handler));
        }

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        if self.providers.is_empty() {
            PluginHealth::Degraded("no providers configured".into())
        } else {
            PluginHealth::Healthy
        }
    }
}
