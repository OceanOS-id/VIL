//! VIL pattern HTTP handlers for the Agent plugin.

use vil_server::prelude::*;
use serde::{Deserialize, Serialize};

use crate::extractors::AgentHandle;

/// Combined service state for the Agent plugin.
pub struct AgentServiceState {
    pub agent: AgentHandle,
    pub tools_resp: ToolsResponseBody,
}

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct RunResponseBody {
    pub answer: String,
    pub tool_calls_made: Vec<ToolCallRecord>,
    pub iterations: usize,
}

#[derive(Debug, Serialize)]
pub struct ToolCallRecord {
    pub tool: String,
    pub input: serde_json::Value,
    pub output: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolsResponseBody {
    pub tools: Vec<ToolInfo>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct ClearMemoryResponse {
    pub message: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /run — Execute the agent ReAct loop on a query.
pub async fn run_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<RunResponseBody>> {
    let state = ctx.state::<AgentServiceState>()?;
    let agent = &state.agent;
    let req: RunRequest = body.json().map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.query.trim().is_empty() {
        return Err(VilError::bad_request("query must not be empty"));
    }

    let result = agent
        .run(&req.query)
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    Ok(VilResponse::ok(RunResponseBody {
        answer: result.answer,
        tool_calls_made: result
            .tool_calls_made
            .iter()
            .map(|tc| ToolCallRecord {
                tool: tc.tool.clone(),
                input: tc.input.clone(),
                output: tc.output.clone(),
            })
            .collect(),
        iterations: result.iterations,
    }))
}

/// GET /tools — List available tools.
pub async fn tools_handler(
    ctx: ServiceCtx,
) -> VilResponse<ToolsResponseBody> {
    let state = ctx.state::<AgentServiceState>().expect("AgentServiceState");
    VilResponse::ok(state.tools_resp.clone())
}

/// POST /memory/clear — Clear conversation memory.
pub async fn clear_memory_handler(
    ctx: ServiceCtx,
) -> VilResponse<ClearMemoryResponse> {
    let state = ctx.state::<AgentServiceState>().expect("AgentServiceState");
    state.agent.memory().clear().await;
    VilResponse::ok(ClearMemoryResponse {
        message: "memory cleared".into(),
    })
}
