use vil_server::prelude::*;

use crate::handlers;
use crate::vil_semantic::{ParseEvent, ParseFault, ParserState};

pub struct OutputParserPlugin;

impl OutputParserPlugin {
    pub fn new() -> Self { Self }
}

impl Default for OutputParserPlugin {
    fn default() -> Self { Self::new() }
}

impl VilPlugin for OutputParserPlugin {
    fn id(&self) -> &str { "vil-output-parser" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "Structured output parsing for LLM responses (JSON, regex, markdown)" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "parser".into(),
            endpoints: vec![
                EndpointSpec::post("/api/parser/parse"),
                EndpointSpec::get("/api/parser/stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }

    fn register(&self, ctx: &mut PluginContext) {
        let svc = ServiceProcess::new("parser")
            .endpoint(Method::POST, "/parse", post(handlers::parse_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<ParseEvent>()
            .faults::<ParseFault>()
            .manages::<ParserState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
