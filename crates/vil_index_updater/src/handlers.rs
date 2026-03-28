use vil_server::prelude::*;

use crate::IncrementalUpdater;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct IndexUpdaterStatsBody {
    pub pending_count: usize,
    pub should_flush: bool,
    pub batch_size: usize,
    pub operations: Vec<String>,
    pub version: String,
}

pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<IndexUpdaterStatsBody>> {
    let updater = ctx
        .state::<Arc<IncrementalUpdater>>()
        .expect("IncrementalUpdater");
    Ok(VilResponse::ok(IndexUpdaterStatsBody {
        pending_count: updater.pending_count(),
        should_flush: updater.should_flush(),
        batch_size: updater.batch_size,
        operations: vec![
            "insert".into(),
            "delete".into(),
            "update".into(),
            "flush".into(),
        ],
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
