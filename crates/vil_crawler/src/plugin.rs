//! VilPlugin implementation for web crawler integration.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::config::CrawlConfig;
use crate::handlers;
use crate::semantic::{CrawlEvent, CrawlFault, CrawlerState};

/// Web crawler plugin — concurrent BFS crawling with robots.txt support.
pub struct CrawlerPlugin {
    config: Arc<CrawlConfig>,
}

impl CrawlerPlugin {
    pub fn new() -> Self {
        Self {
            config: Arc::new(CrawlConfig::default()),
        }
    }

    /// Create with a custom config.
    pub fn with_config(config: CrawlConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Access the shared config.
    pub fn config(&self) -> &Arc<CrawlConfig> {
        &self.config
    }
}

impl Default for CrawlerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for CrawlerPlugin {
    fn id(&self) -> &str {
        "vil-crawler"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Async web crawler with rate limiting and robots.txt support"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "crawler".into(),
            endpoints: vec![
                EndpointSpec::post("/api/crawler/crawl").with_description("Crawl a website"),
                EndpointSpec::get("/api/crawler/stats").with_description("Crawler stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn register(&self, ctx: &mut PluginContext) {
        let config = Arc::clone(&self.config);

        let svc = ServiceProcess::new("crawler")
            .state(config)
            .endpoint(Method::POST, "/crawl", post(handlers::crawl_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<CrawlEvent>()
            .faults::<CrawlFault>()
            .manages::<CrawlerState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
