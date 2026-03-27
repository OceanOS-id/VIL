use vil_server::prelude::*;

use std::sync::Arc;
use crate::config::ai_platform_routes;
use crate::router::SemanticRouter;
use crate::handlers;
use crate::semantic::{RouteEvent, RouteFault, RouterState};

pub struct SemanticRouterPlugin {
    default_target: String,
}

impl SemanticRouterPlugin {
    pub fn new(default_target: impl Into<String>) -> Self {
        Self { default_target: default_target.into() }
    }
}

impl VilPlugin for SemanticRouterPlugin {
    fn id(&self) -> &str { "vil-semantic-router" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Route queries to specialized models/pipelines based on intent" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "semantic-router".into(),
            endpoints: vec![
                EndpointSpec::get("/api/router/routes"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let router = Arc::new(
            SemanticRouter::builder(&self.default_target)
                .routes(ai_platform_routes())
                .build()
        );
        ctx.provide::<Arc<SemanticRouter>>("semantic-router", router.clone());

        let svc = ServiceProcess::new("semantic-router")
            .endpoint(Method::GET, "/routes", get(handlers::routes_handler))
            .state(router)
            .emits::<RouteEvent>()
            .faults::<RouteFault>()
            .manages::<RouterState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
