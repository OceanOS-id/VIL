use std::sync::Arc;
use vil_server::prelude::*;

use crate::handlers::{self, CompilerStats};
use crate::semantic::{CompileEvent, CompileFault, CompilerState};

pub struct AiCompilerPlugin {
    stats: Arc<CompilerStats>,
}

impl AiCompilerPlugin {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(CompilerStats::new()),
        }
    }

    /// Create with pre-populated compiler stats.
    pub fn with_stats(stats: Arc<CompilerStats>) -> Self {
        Self { stats }
    }

    /// Access the shared compiler stats.
    pub fn stats(&self) -> &Arc<CompilerStats> {
        &self.stats
    }
}

impl Default for AiCompilerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for AiCompilerPlugin {
    fn id(&self) -> &str {
        "vil-ai-compiler"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "AI pipeline compiler for optimized execution plans"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "ai-compiler".into(),
            endpoints: vec![EndpointSpec::get("/api/compiler/stats")],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let stats = Arc::clone(&self.stats);

        let svc = ServiceProcess::new("ai-compiler")
            .state(stats)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<CompileEvent>()
            .faults::<CompileFault>()
            .manages::<CompilerState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
