use crate::collection::Collection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vil_server::prelude::*;

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub vector: Vec<f32>,
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponseBody {
    pub results: Vec<SearchHit>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub score: f32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct IndexRequest {
    pub vector: Vec<f32>,
    pub metadata: serde_json::Value,
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IndexResponseBody {
    pub indexed: bool,
}

#[derive(Debug, Serialize)]
pub struct StatsResponseBody {
    pub total_vectors: usize,
    pub dimension: usize,
}

pub async fn search_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<SearchResponseBody>> {
    let col = ctx.state::<Arc<Collection>>()?;
    let req: SearchRequest = body
        .json()
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    let top_k = req.top_k.unwrap_or(10);
    let results = col.search(&req.vector, top_k);
    let hits: Vec<SearchHit> = results
        .iter()
        .map(|r| SearchHit {
            score: r.score,
            metadata: r.metadata.clone(),
        })
        .collect();
    let count = hits.len();
    Ok(VilResponse::ok(SearchResponseBody {
        results: hits,
        count,
    }))
}

pub async fn index_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<IndexResponseBody>> {
    let col = ctx.state::<Arc<Collection>>()?;
    let req: IndexRequest = body
        .json()
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    col.add(req.vector, req.metadata, req.content);
    Ok(VilResponse::created(IndexResponseBody { indexed: true }))
}

pub async fn stats_handler(ctx: ServiceCtx) -> VilResponse<StatsResponseBody> {
    let col = ctx.state::<Arc<Collection>>().expect("Collection");
    VilResponse::ok(StatsResponseBody {
        total_vectors: col.count(),
        dimension: col.dimension(),
    })
}
