//! VilPlugin implementation for SQL agent integration.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::handlers;
use crate::schema::SchemaRegistry;
use crate::semantic::{SqlAgentEvent, SqlAgentFault, SqlAgentState};

/// SQL agent plugin — natural language to SQL generation.
pub struct SqlAgentPlugin {
    registry: Arc<SchemaRegistry>,
}

impl SqlAgentPlugin {
    pub fn new(registry: Arc<SchemaRegistry>) -> Self {
        Self { registry }
    }
}

impl Default for SqlAgentPlugin {
    fn default() -> Self {
        Self {
            registry: Arc::new(SchemaRegistry::new()),
        }
    }
}

impl VilPlugin for SqlAgentPlugin {
    fn id(&self) -> &str {
        "vil-sql-agent"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Natural language to SQL query generation with injection prevention"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "sql-agent".into(),
            endpoints: vec![
                EndpointSpec::post("/api/sql-agent/generate").with_description("Generate SQL from natural language"),
                EndpointSpec::get("/api/sql-agent/stats").with_description("SQL agent stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn register(&self, ctx: &mut PluginContext) {
        let svc = ServiceProcess::new("sql-agent")
            .state(Arc::clone(&self.registry))
            .endpoint(Method::POST, "/generate", post(handlers::generate_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<SqlAgentEvent>()
            .faults::<SqlAgentFault>()
            .manages::<SqlAgentState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
