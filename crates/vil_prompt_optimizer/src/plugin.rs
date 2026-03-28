use vil_server::prelude::*;

use crate::evaluator::KeywordOverlapEvaluator;
use crate::handlers;
use crate::optimizer::PromptOptimizer;
use crate::strategy::OptimizeStrategy;
use crate::vil_semantic::{OptimizeEvent, OptimizeFault, OptimizerState};
use std::sync::{Arc, RwLock};

pub struct PromptOptimizerPlugin;

impl PromptOptimizerPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PromptOptimizerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for PromptOptimizerPlugin {
    fn id(&self) -> &str {
        "vil-prompt-optimizer"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Automated prompt engineering with candidate evaluation"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "prompt-optimizer".into(),
            endpoints: vec![EndpointSpec::get("/api/prompt-optimizer/stats")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let evaluator = Arc::new(KeywordOverlapEvaluator);
        let optimizer = Arc::new(RwLock::new(PromptOptimizer::new(
            evaluator,
            OptimizeStrategy::GridSearch,
        )));

        let svc = ServiceProcess::new("prompt-optimizer")
            .state(optimizer)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<OptimizeEvent>()
            .faults::<OptimizeFault>()
            .manages::<OptimizerState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
