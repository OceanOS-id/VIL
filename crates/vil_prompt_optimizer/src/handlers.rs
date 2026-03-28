use vil_server::prelude::*;

use crate::PromptOptimizer;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize)]
pub struct OptimizerStatsBody {
    pub candidate_count: usize,
    pub best_template: Option<String>,
    pub best_score: Option<f32>,
    pub strategies: Vec<String>,
    pub version: String,
}

pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<OptimizerStatsBody>> {
    let optimizer = ctx.state::<Arc<RwLock<PromptOptimizer>>>()?;
    let opt = optimizer
        .read()
        .map_err(|_| VilError::internal("lock poisoned"))?;
    let best = opt.best();
    Ok(VilResponse::ok(OptimizerStatsBody {
        candidate_count: opt.candidates.len(),
        best_template: best.map(|c| c.template.clone()),
        best_score: best.map(|c| c.score),
        strategies: vec![
            "grid_search".into(),
            "random_search".into(),
            "bayesian".into(),
        ],
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
