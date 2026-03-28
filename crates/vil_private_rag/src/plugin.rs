use vil_server::prelude::*;

use crate::audit::PrivacyAuditLog;
use crate::handlers;
use crate::vil_semantic::{PrivateRagEvent, PrivateRagFault, PrivateRagState};
use std::sync::{Arc, RwLock};

pub struct PrivateRagPlugin;

impl PrivateRagPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrivateRagPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for PrivateRagPlugin {
    fn id(&self) -> &str {
        "vil-private-rag"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Privacy-preserving RAG with PII redaction and audit logging"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "private-rag".into(),
            endpoints: vec![EndpointSpec::get("/api/private-rag/stats")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let audit_log = Arc::new(RwLock::new(PrivacyAuditLog::new()));

        let svc = ServiceProcess::new("private-rag")
            .state(audit_log)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<PrivateRagEvent>()
            .faults::<PrivateRagFault>()
            .manages::<PrivateRagState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
