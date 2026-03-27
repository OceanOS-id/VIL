//! VIL pattern HTTP handlers for the tokenizer plugin.

use vil_server::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::counter::TokenCounter;

#[derive(Debug, Deserialize)]
pub struct CountRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct CountResponseBody {
    pub token_count: usize,
    pub text_length: usize,
}

#[derive(Debug, Deserialize)]
pub struct TruncateRequest {
    pub text: String,
    pub max_tokens: usize,
}

#[derive(Debug, Serialize)]
pub struct TruncateResponseBody {
    pub text: String,
    pub token_count: usize,
    pub truncated: bool,
}

pub async fn count_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<CountResponseBody>> {
    let counter = ctx.state::<Arc<TokenCounter>>()?;
    let req: CountRequest = body.json().map_err(|e| VilError::bad_request(e.to_string()))?;
    let count = counter.count(&req.text);
    Ok(VilResponse::ok(CountResponseBody {
        token_count: count,
        text_length: req.text.len(),
    }))
}

pub async fn truncate_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<TruncateResponseBody>> {
    let counter = ctx.state::<Arc<TokenCounter>>()?;
    let req: TruncateRequest = body.json().map_err(|e| VilError::bad_request(e.to_string()))?;
    let original_count = counter.count(&req.text);
    let result = crate::truncate::truncate_to_tokens(
        counter.tokenizer(),
        &req.text,
        req.max_tokens,
        crate::truncate::TruncateStrategy::TailDrop,
    );
    let new_count = counter.count(&result);
    Ok(VilResponse::ok(TruncateResponseBody {
        text: result,
        token_count: new_count,
        truncated: original_count > req.max_tokens,
    }))
}
