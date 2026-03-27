use vil_server::prelude::*;
use std::sync::Arc;
use crate::router::SemanticRouter;

#[derive(Debug, Serialize)]
pub struct RouteInfo {
    pub name: String,
    pub target: String,
    pub keywords: Vec<String>,
    pub priority: u32,
}

#[derive(Debug, Serialize)]
pub struct RoutesResponseBody {
    pub routes: Vec<RouteInfo>,
    pub default_target: String,
    pub route_count: usize,
}

pub async fn routes_handler(
    ctx: ServiceCtx,
) -> VilResponse<RoutesResponseBody> {
    let router = ctx.state::<Arc<SemanticRouter>>().expect("SemanticRouter");
    let routes: Vec<RouteInfo> = router.classifier().routes().iter().map(|r| RouteInfo {
        name: r.name.clone(),
        target: r.target.clone(),
        keywords: r.keywords.clone(),
        priority: r.priority,
    }).collect();
    let count = routes.len();
    VilResponse::ok(RoutesResponseBody {
        routes,
        default_target: router.default_target().to_string(),
        route_count: count,
    })
}
