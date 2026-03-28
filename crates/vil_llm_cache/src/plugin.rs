use vil_server::prelude::*;

use crate::handlers;
use crate::semantic::{CacheFault, CacheHitEvent, LlmCacheState};
use crate::{CacheConfig, SemanticCache};
use std::sync::Arc;

pub struct LlmCachePlugin {
    config: CacheConfig,
}

impl LlmCachePlugin {
    pub fn new(config: CacheConfig) -> Self {
        Self { config }
    }
}

impl Default for LlmCachePlugin {
    fn default() -> Self {
        Self {
            config: CacheConfig::default(),
        }
    }
}

impl VilPlugin for LlmCachePlugin {
    fn id(&self) -> &str {
        "vil-llm-cache"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Semantic response cache for LLM responses"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "llm-cache".into(),
            endpoints: vec![EndpointSpec::get("/api/cache/stats")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let cache = Arc::new(SemanticCache::new(self.config.clone()));
        ctx.provide::<Arc<SemanticCache>>("llm-cache", cache.clone());

        let svc = ServiceProcess::new("llm-cache")
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(cache)
            .emits::<CacheHitEvent>()
            .faults::<CacheFault>()
            .manages::<LlmCacheState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
