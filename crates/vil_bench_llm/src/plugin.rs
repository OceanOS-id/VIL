use vil_server::prelude::*;
use std::sync::Arc;

use crate::handlers;
use crate::built_in::{FactBench, LogicBench, MathBench};
use crate::suite::BenchSuite;
use crate::vil_semantic::{BenchEvent, BenchFault, BenchState};

pub struct BenchPlugin {
    suite: Arc<BenchSuite>,
}

impl BenchPlugin {
    pub fn new() -> Self {
        let suite = BenchSuite::new()
            .add(Box::new(MathBench))
            .add(Box::new(LogicBench))
            .add(Box::new(FactBench));
        Self {
            suite: Arc::new(suite),
        }
    }

    /// Create with a custom suite.
    pub fn with_suite(suite: Arc<BenchSuite>) -> Self {
        Self { suite }
    }

    /// Access the shared bench suite.
    pub fn suite(&self) -> &Arc<BenchSuite> {
        &self.suite
    }
}

impl Default for BenchPlugin {
    fn default() -> Self { Self::new() }
}

impl VilPlugin for BenchPlugin {
    fn id(&self) -> &str { "vil-bench-llm" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "LLM benchmarking suite with pluggable benchmarks" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "bench".into(),
            endpoints: vec![
                EndpointSpec::get("/api/bench/stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }

    fn register(&self, ctx: &mut PluginContext) {
        let suite = Arc::clone(&self.suite);

        let svc = ServiceProcess::new("bench")
            .state(suite)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<BenchEvent>()
            .faults::<BenchFault>()
            .manages::<BenchState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
