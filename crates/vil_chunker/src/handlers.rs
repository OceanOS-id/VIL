use vil_server::prelude::*;

use crate::{ChunkStrategy, TextChunk};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ChunkRequest {
    pub text: String,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ChunkResponseBody {
    pub chunks: Vec<TextChunk>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct ChunkStatsBody {
    pub strategies: Vec<String>,
    pub version: String,
}

pub async fn chunk_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ChunkResponseBody>> {
    let chunker = ctx
        .state::<Arc<dyn ChunkStrategy>>()
        .expect("ChunkStrategy");
    let req: ChunkRequest = body.json().expect("invalid JSON");
    if req.text.is_empty() {
        return Err(VilError::bad_request("text must not be empty"));
    }
    let chunks = chunker.chunk(&req.text);
    let count = chunks.len();
    Ok(VilResponse::ok(ChunkResponseBody { chunks, count }))
}

pub async fn stats_handler() -> HandlerResult<VilResponse<ChunkStatsBody>> {
    Ok(VilResponse::ok(ChunkStatsBody {
        strategies: vec![
            "sentence".into(),
            "sliding_window".into(),
            "code".into(),
            "table".into(),
        ],
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
