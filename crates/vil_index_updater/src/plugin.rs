use vil_server::prelude::*;

use crate::handlers;
use crate::updater::IncrementalUpdater;
use crate::vil_semantic::{IndexUpdateEvent, IndexUpdateFault, IndexUpdaterState};
use std::sync::Arc;

pub struct IndexUpdaterPlugin;

impl IndexUpdaterPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IndexUpdaterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for IndexUpdaterPlugin {
    fn id(&self) -> &str {
        "vil-index-updater"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Incremental vector index updates with write-ahead log"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "index-updater".into(),
            endpoints: vec![EndpointSpec::get("/api/index-updater/stats")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let updater = Arc::new(IncrementalUpdater::new(100));

        let svc = ServiceProcess::new("index-updater")
            .state(updater)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<IndexUpdateEvent>()
            .faults::<IndexUpdateFault>()
            .manages::<IndexUpdaterState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
