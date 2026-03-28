//! VilPlugin implementation for Feature Store.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::semantic::{FeatureEvent, FeatureFault, FeatureStoreState};
use crate::store::FeatureStore;

pub struct FeatureStorePlugin {
    store: Arc<FeatureStore>,
}

impl FeatureStorePlugin {
    pub fn new() -> Self {
        Self {
            store: Arc::new(FeatureStore::new()),
        }
    }
    pub fn with_store(store: Arc<FeatureStore>) -> Self {
        Self { store }
    }
}

impl Default for FeatureStorePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for FeatureStorePlugin {
    fn id(&self) -> &str {
        "vil-feature-store"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "Online/offline feature serving with TTL"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "feature-store".into(),
            endpoints: vec![
                EndpointSpec::post("/api/features/get"),
                EndpointSpec::post("/api/features/set"),
                EndpointSpec::get("/api/features/stats"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        ctx.provide::<Arc<FeatureStore>>("feature-store", self.store.clone());

        let svc = ServiceProcess::new("feature-store")
            .state(self.store.clone())
            .emits::<FeatureEvent>()
            .faults::<FeatureFault>()
            .manages::<FeatureStoreState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
