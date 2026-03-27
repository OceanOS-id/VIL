use vil_server::prelude::*;

use std::sync::Arc;

use crate::engine::ConsensusEngine;
use crate::handlers;
use crate::semantic::{ConsensusEvent, ConsensusFault, ConsensusState};
use crate::strategy::ConsensusStrategy;

pub struct ConsensusPlugin {
    engine: Arc<ConsensusEngine>,
}

impl ConsensusPlugin {
    pub fn new(engine: Arc<ConsensusEngine>) -> Self {
        Self { engine }
    }

    /// Create a plugin with an empty provider list and BestOfN strategy (for testing).
    pub fn default_empty() -> Self {
        Self {
            engine: Arc::new(ConsensusEngine::new(vec![], ConsensusStrategy::BestOfN)),
        }
    }
}

impl VilPlugin for ConsensusPlugin {
    fn id(&self) -> &str { "vil-consensus" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Multi-model consensus with parallel inference and voting" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "consensus".into(),
            endpoints: vec![
                EndpointSpec::get("/api/consensus/stats"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let svc = ServiceProcess::new("consensus")
            .state(Arc::clone(&self.engine))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<ConsensusEvent>()
            .faults::<ConsensusFault>()
            .manages::<ConsensusState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
