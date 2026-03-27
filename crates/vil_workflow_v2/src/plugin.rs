//! VilPlugin implementation for workflow orchestration integration.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::handlers;
use crate::scheduler::WorkflowScheduler;
use crate::semantic::{WorkflowEvent, WorkflowFault, WorkflowState};

/// Workflow plugin — DAG-based AI workflow orchestration.
pub struct WorkflowPlugin;

impl WorkflowPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorkflowPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for WorkflowPlugin {
    fn id(&self) -> &str {
        "vil-workflow"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "DAG-based AI workflow orchestration with parallel task execution"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "workflow".into(),
            endpoints: vec![
                EndpointSpec::get("/api/workflow/stats").with_description("Workflow stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn register(&self, ctx: &mut PluginContext) {
        let scheduler = Arc::new(WorkflowScheduler::new());

        let svc = ServiceProcess::new("workflow")
            .state(scheduler)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<WorkflowEvent>()
            .faults::<WorkflowFault>()
            .manages::<WorkflowState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
