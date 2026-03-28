//! VilPlugin implementation for cost tracking integration.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::handlers;
use crate::semantic::{CostEvent, CostFault, CostState};
use crate::tracker::CostTracker;

/// Cost tracker plugin — per-request cost tracking across providers.
pub struct CostTrackerPlugin {
    tracker: Arc<CostTracker>,
}

impl CostTrackerPlugin {
    pub fn new() -> Self {
        Self {
            tracker: Arc::new(CostTracker::new()),
        }
    }

    /// Create with a pre-existing tracker.
    pub fn with_tracker(tracker: Arc<CostTracker>) -> Self {
        Self { tracker }
    }

    /// Access the shared cost tracker.
    pub fn tracker(&self) -> &Arc<CostTracker> {
        &self.tracker
    }
}

impl Default for CostTrackerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for CostTrackerPlugin {
    fn id(&self) -> &str {
        "vil-cost-tracker"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Per-request cost tracking across LLM providers with budget enforcement"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "cost".into(),
            endpoints: vec![
                EndpointSpec::get("/api/cost/stats").with_description("Cost tracking stats")
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn register(&self, ctx: &mut PluginContext) {
        let tracker = Arc::clone(&self.tracker);

        let svc = ServiceProcess::new("cost-tracker")
            .state(tracker)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<CostEvent>()
            .faults::<CostFault>()
            .manages::<CostState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
