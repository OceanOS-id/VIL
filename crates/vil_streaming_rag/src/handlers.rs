// =============================================================================
// VIL REST Handlers — Streaming RAG
// =============================================================================

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use vil_server::prelude::*;

use crate::stream::{compute_embedding, StreamingIngester};

// ── Request / Response types ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct QueryResultItem {
    pub text: String,
    pub index: usize,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub query: String,
    pub results: Vec<QueryResultItem>,
}

#[derive(Debug, Serialize)]
pub struct StreamingRagStatsResponse {
    pub chunk_count: usize,
    pub buffer_len: usize,
    pub chunk_size: usize,
    pub overlap: usize,
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/streaming-rag/query — search the index with a natural-language query.
pub async fn handle_query(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<QueryResponse>> {
    let ingester = ctx
        .state::<Arc<StreamingIngester>>()
        .expect("StreamingIngester");
    let req: QueryRequest = body.json().expect("invalid JSON");
    let top_k = req.top_k.unwrap_or(5);
    let embedding = compute_embedding(&req.query);
    let results = ingester
        .search(&embedding, top_k)
        .into_iter()
        .map(|r| QueryResultItem {
            text: r.text,
            index: r.index,
            score: r.score,
        })
        .collect();
    let resp = QueryResponse {
        query: req.query,
        results,
    };
    Ok(VilResponse::ok(resp))
}

/// GET /api/streaming-rag/stats — return pipeline statistics.
pub async fn handle_stats(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<StreamingRagStatsResponse>> {
    let ingester = ctx
        .state::<Arc<StreamingIngester>>()
        .expect("StreamingIngester");
    let config = ingester.config();
    let resp = StreamingRagStatsResponse {
        chunk_count: ingester.chunk_count(),
        buffer_len: ingester.buffer_len(),
        chunk_size: config.chunk_size,
        overlap: config.overlap,
    };
    Ok(VilResponse::ok(resp))
}
