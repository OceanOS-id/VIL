use crate::pipeline::RealtimeRagPipeline;
use std::sync::Arc;
use vil_server::prelude::*;

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub embedding: Vec<f32>,
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct QueryResponseBody {
    pub chunks: Vec<ChunkHit>,
    pub count: usize,
    pub from_cache: bool,
}

#[derive(Debug, Serialize)]
pub struct ChunkHit {
    pub doc_id: String,
    pub content: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct StatsResponseBody {
    pub doc_count: usize,
    pub cache_size: usize,
}

pub async fn query_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<QueryResponseBody>> {
    let pipeline = ctx.state::<Arc<RealtimeRagPipeline>>()?;
    let req: QueryRequest = body
        .json()
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    let result = pipeline.query_with_embedding(&req.embedding);
    let chunks: Vec<ChunkHit> = result
        .chunks
        .iter()
        .map(|r| ChunkHit {
            doc_id: r.doc_id.clone(),
            content: r.text.clone(),
            score: r.score,
        })
        .collect();
    let count = chunks.len();
    Ok(VilResponse::ok(QueryResponseBody {
        chunks,
        count,
        from_cache: false,
    }))
}

pub async fn stats_handler(ctx: ServiceCtx) -> VilResponse<StatsResponseBody> {
    let pipeline = ctx
        .state::<Arc<RealtimeRagPipeline>>()
        .expect("RealtimeRagPipeline");
    VilResponse::ok(StatsResponseBody {
        doc_count: pipeline.doc_count(),
        cache_size: pipeline.cache_size(),
    })
}
