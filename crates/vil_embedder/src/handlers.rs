use vil_server::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::provider::EmbedProvider;

#[derive(Debug, Deserialize)]
pub struct EmbedRequest { pub texts: Vec<String> }

#[derive(Debug, Serialize)]
pub struct EmbedResponseBody {
    pub embeddings: Vec<Vec<f32>>,
    pub dimension: usize,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct SimilarityRequest { pub a: Vec<f32>, pub b: Vec<f32> }

#[derive(Debug, Serialize)]
pub struct SimilarityResponseBody { pub cosine: f32 }

pub async fn embed_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<EmbedResponseBody>> {
    let provider = ctx.state::<Arc<dyn EmbedProvider>>().expect("EmbedProvider");
    let req: EmbedRequest = body.json().map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.texts.is_empty() {
        return Err(VilError::bad_request("texts must not be empty"));
    }
    let embeddings = provider.embed_batch(&req.texts).await
        .map_err(|e| VilError::internal(e.to_string()))?;
    let dim = embeddings.first().map(|e| e.len()).unwrap_or(0);
    let count = embeddings.len();
    Ok(VilResponse::ok(EmbedResponseBody { embeddings, dimension: dim, count }))
}

pub async fn similarity_handler(
    body: ShmSlice,
) -> HandlerResult<VilResponse<SimilarityResponseBody>> {
    let req: SimilarityRequest = body.json().map_err(|e| VilError::bad_request(e.to_string()))?;
    let sim = crate::similarity::cosine_similarity(&req.a, &req.b);
    Ok(VilResponse::ok(SimilarityResponseBody { cosine: sim }))
}
