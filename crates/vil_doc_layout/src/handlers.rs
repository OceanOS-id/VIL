//! HTTP handlers for the doc layout plugin — wired to real LayoutAnalyzer state.

use vil_server::prelude::*;
use std::sync::Arc;

use crate::analyzer::LayoutAnalyzer;
use crate::element::LayoutRegion;

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponseBody {
    pub regions: Vec<LayoutRegion>,
    pub count: usize,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /analyze — analyze text layout using the shared LayoutAnalyzer.
pub async fn analyze_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<AnalyzeResponseBody>> {
    let analyzer = ctx.state::<Arc<LayoutAnalyzer>>().expect("LayoutAnalyzer");
    let req: AnalyzeRequest = body.json().expect("invalid JSON");
    if req.text.is_empty() {
        return Err(VilError::bad_request("text must not be empty"));
    }
    let regions = analyzer.analyze(&req.text);
    let count = regions.len();
    Ok(VilResponse::ok(AnalyzeResponseBody { regions, count }))
}
