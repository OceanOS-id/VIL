use std::sync::Arc;
use vil_server::prelude::*;

use crate::handlers;
use crate::tracer::AiTracer;
use crate::vil_semantic::{TraceEvent, TraceFault, TraceState};

pub struct AiTracePlugin {
    tracer: Arc<AiTracer>,
}

impl AiTracePlugin {
    pub fn new() -> Self {
        Self {
            tracer: Arc::new(AiTracer::new()),
        }
    }

    /// Create with a pre-existing tracer.
    pub fn with_tracer(tracer: Arc<AiTracer>) -> Self {
        Self { tracer }
    }

    /// Access the shared tracer.
    pub fn tracer(&self) -> &Arc<AiTracer> {
        &self.tracer
    }
}

impl Default for AiTracePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for AiTracePlugin {
    fn id(&self) -> &str {
        "vil-ai-trace"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Distributed tracing for AI pipelines with metrics export"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "trace".into(),
            endpoints: vec![EndpointSpec::get("/api/trace/stats")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let tracer = Arc::clone(&self.tracer);

        let svc = ServiceProcess::new("trace")
            .state(tracer)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<TraceEvent>()
            .faults::<TraceFault>()
            .manages::<TraceState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
