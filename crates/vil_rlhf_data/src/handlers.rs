use vil_server::prelude::*;

use crate::dataset::{DatasetStats, PreferenceDataset};
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize)]
pub struct RlhfStatsBody {
    pub pair_count: usize,
    pub dataset_stats: DatasetStats,
    pub formats: Vec<String>,
    pub version: String,
}

pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<RlhfStatsBody>> {
    let dataset = ctx.state::<Arc<RwLock<PreferenceDataset>>>()?;
    let ds = dataset
        .read()
        .map_err(|_| VilError::internal("lock poisoned"))?;
    Ok(VilResponse::ok(RlhfStatsBody {
        pair_count: ds.len(),
        dataset_stats: ds.stats(),
        formats: vec!["dpo".into(), "rlhf".into()],
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
