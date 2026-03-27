use vil_server::prelude::*;

use crate::{RerankCandidate, RerankResult, Reranker};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct RerankRequest {
    pub query: String,
    pub candidates: Vec<RerankCandidate>,
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct RerankResponseBody {
    pub results: Vec<RerankResult>,
    pub count: usize,
}

pub async fn rerank_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<RerankResponseBody>> {
    let reranker = ctx.state::<Arc<dyn Reranker>>().expect("Reranker");
    let req: RerankRequest = body.json().expect("invalid JSON");
    if req.candidates.is_empty() {
        return Err(VilError::bad_request("candidates must not be empty"));
    }
    let top_k = req.top_k.unwrap_or(req.candidates.len());
    let results = reranker.rerank(&req.query, &req.candidates, top_k).await
        .map_err(|e| VilError::internal(e.to_string()))?;
    let count = results.len();
    Ok(VilResponse::ok(RerankResponseBody { results, count }))
}
