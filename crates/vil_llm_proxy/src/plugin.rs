//! VilPlugin implementation for LLM proxy.
use vil_server::prelude::*;

use std::sync::Arc;
use crate::proxy::LlmProxy;
use crate::handlers::{self, LlmProxyState};
use crate::semantic::{ProxyRequestEvent, ProxyFault, ProxyState};

pub struct LlmProxyPlugin;

impl LlmProxyPlugin {
    pub fn new() -> Self { Self }
}

impl Default for LlmProxyPlugin {
    fn default() -> Self { Self::new() }
}

impl VilPlugin for LlmProxyPlugin {
    fn id(&self) -> &str { "vil-llm-proxy" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Rate-limited LLM proxy with caching and routing" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "llm-proxy".into(),
            endpoints: vec![
                EndpointSpec::post("/api/proxy/chat"),
                EndpointSpec::get("/api/proxy/stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![PluginDependency::required("vil-llm", ">=0.1")]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let llm = ctx.require::<Arc<dyn vil_llm::LlmProvider>>("llm").clone();
        let mut proxy = LlmProxy::new();
        proxy.set_provider(llm);
        let metrics = proxy.metrics().clone();
        let proxy = Arc::new(proxy);

        ctx.provide::<Arc<LlmProxy>>("llm-proxy", proxy.clone());

        let svc = ServiceProcess::new("llm-proxy")
            .endpoint(Method::POST, "/chat", post(handlers::proxy_chat_handler))
            .endpoint(Method::GET, "/stats", get(handlers::proxy_stats_handler))
            .state(LlmProxyState { proxy, metrics })
            .emits::<ProxyRequestEvent>()
            .faults::<ProxyFault>()
            .manages::<ProxyState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
