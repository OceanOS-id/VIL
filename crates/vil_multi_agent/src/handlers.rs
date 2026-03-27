// =============================================================================
// VIL REST Handlers — Multi-Agent
// =============================================================================

use std::sync::Arc;

use vil_server::prelude::*;

use crate::orchestrator::Orchestrator;

use tokio::sync::Mutex;

// ── Request / Response types ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub final_answer: String,
    pub agent_outputs: Vec<(String, String)>,
    pub total_ms: u64,
    pub messages_exchanged: usize,
}

#[derive(Debug, Serialize)]
pub struct MultiAgentStatsResponse {
    pub agent_count: usize,
    pub edge_count: usize,
    pub timeout_ms: u64,
    pub max_messages: usize,
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/multi-agent/run — execute the agent graph with a query.
pub async fn handle_run(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<RunResponse>> {
    let orchestrator = ctx.state::<Arc<Mutex<Orchestrator>>>().expect("Orchestrator");
    let req: RunRequest = body.json().expect("invalid JSON");
    let mut orch = orchestrator.lock().await;
    match orch.run(&req.query).await {
        Ok(result) => {
            let resp = RunResponse {
                final_answer: result.final_answer,
                agent_outputs: result.agent_outputs,
                total_ms: result.total_ms,
                messages_exchanged: result.messages_exchanged,
            };
            Ok(VilResponse::ok(resp))
        }
        Err(e) => Err(VilError::internal(e.to_string())),
    }
}

/// GET /api/multi-agent/stats — return orchestrator statistics.
pub async fn handle_stats(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<MultiAgentStatsResponse>> {
    let orchestrator = ctx.state::<Arc<Mutex<Orchestrator>>>().expect("Orchestrator");
    let orch = orchestrator.lock().await;
    let resp = MultiAgentStatsResponse {
        agent_count: orch.graph.agent_count(),
        edge_count: orch.graph.edges.len(),
        timeout_ms: orch.config.timeout_ms,
        max_messages: orch.config.max_messages,
    };
    Ok(VilResponse::ok(resp))
}
