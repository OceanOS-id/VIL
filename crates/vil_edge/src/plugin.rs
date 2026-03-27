use vil_server::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::EdgeConfig;
use crate::handlers;
use crate::runtime::EdgeRuntime;
use crate::vil_semantic::{EdgeEvent, EdgeFault, EdgeState};

pub struct EdgePlugin {
    runtime: Arc<Mutex<EdgeRuntime>>,
}

impl EdgePlugin {
    pub fn new() -> Self {
        Self {
            runtime: Arc::new(Mutex::new(EdgeRuntime::new(EdgeConfig::default()))),
        }
    }

    /// Create with a pre-configured runtime.
    pub fn with_runtime(runtime: Arc<Mutex<EdgeRuntime>>) -> Self {
        Self { runtime }
    }

    /// Access the shared runtime.
    pub fn runtime(&self) -> &Arc<Mutex<EdgeRuntime>> {
        &self.runtime
    }
}

impl Default for EdgePlugin {
    fn default() -> Self { Self::new() }
}

impl VilPlugin for EdgePlugin {
    fn id(&self) -> &str { "vil-edge" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "Edge-optimized inference runtime for constrained devices" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "edge".into(),
            endpoints: vec![
                EndpointSpec::get("/api/edge/stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }

    fn register(&self, ctx: &mut PluginContext) {
        let runtime = Arc::clone(&self.runtime);

        let svc = ServiceProcess::new("edge")
            .state(runtime)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<EdgeEvent>()
            .faults::<EdgeFault>()
            .manages::<EdgeState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
