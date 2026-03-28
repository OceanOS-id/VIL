//! VilPlugin implementation for tokenizer.

use vil_server::prelude::*;

use crate::counter::TokenCounter;
use crate::handlers;
use crate::semantic::{TokenizeEvent, TokenizeFault, TokenizerState};
use std::sync::Arc;

pub struct TokenizerPlugin;

impl TokenizerPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TokenizerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for TokenizerPlugin {
    fn id(&self) -> &str {
        "vil-tokenizer"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "BPE tokenizer: count, truncate, encode/decode"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "tokenizer".into(),
            endpoints: vec![
                EndpointSpec::post("/api/tokenizer/count"),
                EndpointSpec::post("/api/tokenizer/truncate"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let counter = Arc::new(TokenCounter::gpt4());
        ctx.provide::<Arc<TokenCounter>>("tokenizer", counter.clone());

        let svc = ServiceProcess::new("tokenizer")
            .endpoint(Method::POST, "/count", post(handlers::count_handler))
            .endpoint(Method::POST, "/truncate", post(handlers::truncate_handler))
            .state(counter)
            .emits::<TokenizeEvent>()
            .faults::<TokenizeFault>()
            .manages::<TokenizerState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
