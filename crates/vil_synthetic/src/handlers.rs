use vil_server::prelude::*;

use std::sync::Arc;

use crate::generator::SyntheticGenerator;

#[derive(Debug, Serialize)]
pub struct SyntheticStatsBody {
    pub template_count: usize,
    pub templates: Vec<String>,
    pub version: String,
}

pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<SyntheticStatsBody>> {
    let gen = ctx
        .state::<Arc<SyntheticGenerator>>()
        .expect("SyntheticGenerator");
    let templates: Vec<String> = gen.templates.iter().map(|t| t.name.clone()).collect();
    let template_count = templates.len();
    Ok(VilResponse::ok(SyntheticStatsBody {
        template_count,
        templates,
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
