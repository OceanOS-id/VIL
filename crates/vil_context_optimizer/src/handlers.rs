//! VIL pattern HTTP handlers for the Context Optimizer plugin.
//!
//! All handlers follow VIL conventions:
//! - Extract shared state via `ServiceCtx`
//! - Return `HandlerResult<VilResponse<T>>` or `VilResponse<T>`
//! - Use `VilError` for structured error responses

use vil_server::prelude::*;
use serde::{Deserialize, Serialize};

use crate::budget::TokenBudget;
use crate::optimizer::ContextOptimizer;
use crate::strategy::OptimizeStrategy;
use crate::semantic::{OptimizeEvent, OptimizerState};

use std::sync::{Arc, Mutex};

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OptimizeRequest {
    pub chunks: Vec<ChunkInput>,
    #[serde(default = "default_budget")]
    pub budget: usize,
    #[serde(default)]
    pub strategy: Option<String>,
}

fn default_budget() -> usize {
    8000
}

#[derive(Debug, Deserialize)]
pub struct ChunkInput {
    pub text: String,
    #[serde(default = "default_score")]
    pub score: f32,
}

fn default_score() -> f32 {
    0.5
}

#[derive(Debug, Serialize)]
pub struct OptimizeResponseBody {
    pub chunks: Vec<String>,
    pub original_count: usize,
    pub final_count: usize,
    pub tokens_saved: usize,
    pub compression_ratio: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimizerStatsBody {
    pub total_optimizations: u64,
    pub total_tokens_saved: u64,
    pub total_chunks_processed: u64,
    pub avg_compression_ratio: f64,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /optimize — Optimize context chunks to fit token budget.
pub async fn optimize_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<OptimizeResponseBody>> {
    let state = ctx.state::<Arc<Mutex<OptimizerState>>>().expect("OptimizerState");
    let req: OptimizeRequest = body.json().expect("invalid JSON");
    if req.chunks.is_empty() {
        return Err(VilError::bad_request("chunks must not be empty"));
    }

    let budget = TokenBudget::new(req.budget);
    let strategy = match req.strategy.as_deref() {
        Some("dedup") => OptimizeStrategy::DedupAndFit { dedup_threshold: 0.8 },
        Some("full") => OptimizeStrategy::Full { dedup_threshold: 0.8 },
        Some("budget") => OptimizeStrategy::BudgetFit,
        _ => OptimizeStrategy::Full { dedup_threshold: 0.8 },
    };

    let optimizer = ContextOptimizer::new(budget).strategy(strategy);
    let input: Vec<(String, f32)> = req.chunks.iter().map(|c| (c.text.clone(), c.score)).collect();
    let result = optimizer.optimize(&input);

    let strategy_name = req.strategy.unwrap_or_else(|| "full".into());
    let event = OptimizeEvent {
        original_count: result.original_count,
        final_count: result.final_count,
        tokens_saved: result.tokens_saved,
        compression_ratio: result.compression_ratio,
        strategy: strategy_name,
    };

    if let Ok(mut s) = state.lock() {
        s.record(&event);
    }

    Ok(VilResponse::ok(OptimizeResponseBody {
        chunks: result.chunks,
        original_count: result.original_count,
        final_count: result.final_count,
        tokens_saved: result.tokens_saved,
        compression_ratio: result.compression_ratio,
    }))
}

/// GET /stats — Get optimizer statistics.
pub async fn stats_handler(
    ctx: ServiceCtx,
) -> VilResponse<OptimizerStatsBody> {
    let state = ctx.state::<Arc<Mutex<OptimizerState>>>().expect("OptimizerState");
    let s = state.lock().unwrap_or_else(|e| e.into_inner());
    VilResponse::ok(OptimizerStatsBody {
        total_optimizations: s.total_optimizations,
        total_tokens_saved: s.total_tokens_saved,
        total_chunks_processed: s.total_chunks_processed,
        avg_compression_ratio: s.avg_compression_ratio,
    })
}
