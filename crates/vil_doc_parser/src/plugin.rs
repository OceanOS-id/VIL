use vil_server::prelude::*;

use crate::handlers;
use crate::semantic::{ParseEvent, ParseFault, DocParserState};

pub struct DocParserPlugin;

impl DocParserPlugin {
    pub fn new() -> Self { Self }
}

impl Default for DocParserPlugin {
    fn default() -> Self { Self::new() }
}

impl VilPlugin for DocParserPlugin {
    fn id(&self) -> &str { "vil-doc-parser" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "Multi-format document parsing for RAG pipelines" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "doc-parser".into(),
            endpoints: vec![
                EndpointSpec::post("/api/parser/parse"),
                EndpointSpec::get("/api/parser/formats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }

    fn register(&self, ctx: &mut PluginContext) {
        let svc = ServiceProcess::new("doc-parser")
            .endpoint(Method::POST, "/parse", post(handlers::parse_handler))
            .endpoint(Method::GET, "/formats", get(handlers::formats_handler))
            .emits::<ParseEvent>()
            .faults::<ParseFault>()
            .manages::<DocParserState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
