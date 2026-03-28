//! VIL pattern HTTP handlers for the Prompt Shield plugin.
//!
//! All handlers follow VIL conventions:
//! - Extract shared state via `Extension<T>`
//! - Return `HandlerResult<VilResponse<T>>` or `VilResponse<T>`
//! - Use `VilError` for structured error responses

use serde::{Deserialize, Serialize};
use vil_server::prelude::*;

use crate::detector::PromptShield;
use crate::result::ScanResult;
use crate::semantic::{ShieldEvent, ShieldState};

use std::sync::{Arc, Mutex};

/// Combined service state for the Prompt Shield plugin.
pub struct ShieldServiceState {
    pub shield: Arc<PromptShield>,
    pub state: Arc<Mutex<ShieldState>>,
}

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct ScanResponseBody {
    pub safe: bool,
    pub risk_level: String,
    pub score: f64,
    pub threat_count: usize,
    pub scan_time_us: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShieldStatsBody {
    pub total_scans: u64,
    pub total_blocked: u64,
    pub total_safe: u64,
    pub avg_scan_time_us: f64,
    pub pattern_count: usize,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /scan — Scan text for prompt injection threats.
pub async fn scan_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ScanResponseBody>> {
    let svc = ctx.state::<ShieldServiceState>()?;
    let shield = &svc.shield;
    let state = &svc.state;
    let req: ScanRequest = body
        .json()
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.text.is_empty() {
        return Err(VilError::bad_request("text must not be empty"));
    }

    let result: ScanResult = shield.scan(&req.text);

    let event = ShieldEvent {
        input_length: req.text.len(),
        safe: result.safe,
        risk_score: result.score,
        threat_count: result.threats.len(),
        scan_time_us: result.scan_time_us,
    };

    if let Ok(mut s) = state.lock() {
        s.record(&event);
    }

    Ok(VilResponse::ok(ScanResponseBody {
        safe: result.safe,
        risk_level: format!("{:?}", result.risk_level),
        score: result.score,
        threat_count: result.threats.len(),
        scan_time_us: result.scan_time_us,
    }))
}

/// GET /stats — Get shield scan statistics.
pub async fn stats_handler(ctx: ServiceCtx) -> VilResponse<ShieldStatsBody> {
    let svc = ctx
        .state::<ShieldServiceState>()
        .expect("ShieldServiceState");
    let shield = &svc.shield;
    let s = svc.state.lock().unwrap_or_else(|e| e.into_inner());
    VilResponse::ok(ShieldStatsBody {
        total_scans: s.total_scans,
        total_blocked: s.total_blocked,
        total_safe: s.total_safe,
        avg_scan_time_us: s.avg_scan_time_us,
        pattern_count: shield.pattern_count(),
    })
}
