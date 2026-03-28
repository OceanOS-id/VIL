use vil_server::prelude::*;

use crate::{CacheStats, SemanticCache};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct CacheStatsResponseBody {
    pub stats: CacheStats,
    pub version: String,
}

pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<CacheStatsResponseBody>> {
    let cache = ctx.state::<Arc<SemanticCache>>().expect("SemanticCache");
    let stats = cache.stats();
    Ok(VilResponse::ok(CacheStatsResponseBody {
        stats,
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
