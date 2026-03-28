use vil_server::prelude::*;

use crate::FusionEngine;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct MultimodalStatsBody {
    pub modalities: Vec<String>,
    pub fusion_strategies: Vec<String>,
    pub default_weights: Vec<f32>,
    pub default_weights_count: usize,
    pub version: String,
}

pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<MultimodalStatsBody>> {
    let engine = ctx.state::<Arc<FusionEngine>>().expect("FusionEngine");
    Ok(VilResponse::ok(MultimodalStatsBody {
        modalities: vec![
            "text".into(),
            "image".into(),
            "audio".into(),
            "video".into(),
        ],
        fusion_strategies: vec!["weighted_average".into(), "concatenation".into()],
        default_weights: engine.default_weights.clone(),
        default_weights_count: engine.default_weights.len(),
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
