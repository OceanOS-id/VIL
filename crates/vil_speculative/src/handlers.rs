use vil_server::prelude::*;

use crate::config::SpeculativeConfig;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct StatsResponseBody {
    pub max_draft_tokens: usize,
    pub max_total_tokens: usize,
    pub max_iterations: usize,
    pub version: String,
}

pub async fn stats_handler(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<StatsResponseBody>> {
    let config = ctx.state::<Arc<SpeculativeConfig>>().expect("SpeculativeConfig");
    Ok(VilResponse::ok(StatsResponseBody {
        max_draft_tokens: config.max_draft_tokens,
        max_total_tokens: config.max_total_tokens,
        max_iterations: config.max_iterations,
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
