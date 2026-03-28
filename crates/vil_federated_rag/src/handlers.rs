use vil_server::prelude::*;

use crate::FederatedRetriever;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct FederatedStatsBody {
    pub source_count: usize,
    pub merge_strategies: Vec<String>,
    pub tolerate_failures: bool,
    pub max_results: usize,
    pub version: String,
}

pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<FederatedStatsBody>> {
    let retriever = ctx
        .state::<Arc<FederatedRetriever>>()
        .expect("FederatedRetriever");
    Ok(VilResponse::ok(FederatedStatsBody {
        source_count: retriever.sources.len(),
        merge_strategies: vec![
            "score_interleave".into(),
            "round_robin".into(),
            "dedup".into(),
        ],
        tolerate_failures: retriever.config.tolerate_failures,
        max_results: retriever.config.max_results,
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
