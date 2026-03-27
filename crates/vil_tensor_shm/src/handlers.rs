use vil_server::prelude::*;
use std::sync::Arc;
use crate::pool::TensorPool;

#[derive(Debug, Serialize)]
pub struct StatsResponseBody {
    pub buffer_count: usize,
    pub is_empty: bool,
}

pub async fn stats_handler(
    ctx: ServiceCtx,
) -> VilResponse<StatsResponseBody> {
    let pool = ctx.state::<Arc<TensorPool>>().expect("TensorPool");
    VilResponse::ok(StatsResponseBody {
        buffer_count: pool.len(),
        is_empty: pool.is_empty(),
    })
}
