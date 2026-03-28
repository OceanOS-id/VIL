use vil_server::prelude::*;

use std::sync::Arc;

use crate::engine::ConsensusEngine;

#[derive(Debug, Serialize)]
pub struct StatsResponseBody {
    pub provider_count: usize,
    pub strategy: String,
    pub description: String,
}

pub async fn stats_handler(ctx: ServiceCtx) -> VilResponse<StatsResponseBody> {
    let engine = ctx
        .state::<Arc<ConsensusEngine>>()
        .expect("ConsensusEngine");
    VilResponse::ok(StatsResponseBody {
        provider_count: engine.provider_count(),
        strategy: engine.strategy_name(),
        description: "Multi-model consensus with parallel inference and voting/fusion".into(),
    })
}
