//! VIL pattern HTTP handlers for the RAG plugin.

use serde::{Deserialize, Serialize};
use vil_server::prelude::*;

use crate::extractors::Rag;

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub doc_id: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct IngestResponseBody {
    pub doc_id: String,
    pub chunks_stored: usize,
}

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub question: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_top_k() -> usize {
    5
}

#[derive(Debug, Serialize)]
pub struct QueryResponseBody {
    pub answer: String,
    pub sources: Vec<SourceRef>,
}

#[derive(Debug, Serialize)]
pub struct SourceRef {
    pub doc_id: String,
    pub content: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsResponseBody {
    pub status: String,
    pub chunk_count: usize,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /ingest — Ingest a document (chunk -> embed -> store).
pub async fn ingest_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<IngestResponseBody>> {
    let rag = ctx.state::<Rag>()?;
    let req: IngestRequest = body
        .json()
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.content.trim().is_empty() {
        return Err(VilError::bad_request("content must not be empty"));
    }
    if req.doc_id.trim().is_empty() {
        return Err(VilError::bad_request("doc_id must not be empty"));
    }

    let result = rag
        .ingest(&req.doc_id, &req.content)
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    Ok(VilResponse::created(IngestResponseBody {
        doc_id: result.doc_id,
        chunks_stored: result.chunks_stored,
    }))
}

/// POST /query — RAG query (retrieve context + generate answer).
pub async fn query_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<QueryResponseBody>> {
    let rag = ctx.state::<Rag>()?;
    let req: QueryRequest = body
        .json()
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.question.trim().is_empty() {
        return Err(VilError::bad_request("question must not be empty"));
    }

    let result = rag
        .query(&req.question)
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    Ok(VilResponse::ok(QueryResponseBody {
        answer: result.answer,
        sources: result
            .sources
            .iter()
            .map(|s| SourceRef {
                doc_id: s.doc_id.clone(),
                content: s.content.clone(),
                score: s.score,
            })
            .collect(),
    }))
}

/// GET /stats — RAG index stats.
pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<StatsResponseBody>> {
    let rag = ctx.state::<Rag>()?;
    let store = rag.store();
    let count = store
        .count()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    Ok(VilResponse::ok(StatsResponseBody {
        status: "ok".into(),
        chunk_count: count,
    }))
}
