// ╔════════════════════════════════════════════════════════════╗
// ║  401 — Financial Calculator Agent                         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Finance — Computation Agent                     ║
// ║  Pattern:  VX_APP                                        ║
// ║  Token:    N/A                                           ║
// ║  Unique:   Tool schema sent to LLM. Local eval pending.   ║
// ║            local expression evaluation, no multi-turn    ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: AI agent for financial computations. Handles   ║
// ║  loan amortization, compound interest, ROI, and NPV       ║
// ║  calculations. The LLM autonomously decides when to       ║
// ║  invoke the calculator tool vs. answering directly.       ║
// ║  Use case: financial advisors, internal finance tools.    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p agent-plugin-usage-calculator
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What is 42 * 13 + 7?"}' \
//     http://localhost:3120/api/calc

use vil_agent::semantic::{AgentCompletionEvent, AgentFault, AgentMemoryState};
use vil_server::prelude::*;

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct CalcAgentState {
    pub total_queries: u64,
    pub expressions_evaluated: u64,
    pub errors: u64,
}

#[derive(Clone, Debug)]
pub struct CalcToolEvent {
    pub expression: String,
    pub result: String,
    pub success: bool,
}

#[vil_fault]
pub enum CalcAgentFault {
    InvalidExpression,
    DivisionByZero,
    LlmUpstreamError,
}

// ── Request / Response ──────────────────────────────────────────────
// Financial advisors submit natural language math questions (e.g.,
// "What is the monthly payment on a $250K loan at 6.5% over 30 years?").
// The agent decides whether to use the calculator tool or answer directly.

/// Financial calculation request — natural language math question
#[derive(Debug, Deserialize)]
struct CalcRequest {
    prompt: String, // e.g., "Calculate compound interest on $10K at 5% for 3 years"
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct CalcResponse {
    content: String,
    tools_used: Vec<String>,
}

// ── System prompt describing available tools ────────────────────────
// The system prompt defines the agent's tools and usage instructions.
// The LLM autonomously decides when to invoke the calculator tool
// vs. answering from its own knowledge (e.g., "What is 15% of 240?").

const SYSTEM_PROMPT: &str = "\
You are a calculator agent. You have the following tool available:

- calculator: Evaluate arithmetic and algebraic expressions.
  Usage: Pass any math expression (e.g. \"42 * 13 + 7\", \"sqrt(144)\", \"2^10\").
  The calculator supports +, -, *, /, ^, sqrt, sin, cos, tan, log, abs, and parentheses.

When the user asks a math question, use the calculator tool to compute the answer.
Always show the expression you evaluated and the result.";

// ── Handler: SSE collect ────────────────────────────────────────────
// Forwards the financial calculation question to the LLM with tool context.
// The LLM may invoke the calculator tool or answer directly depending on
// the complexity of the question.

/// POST /api/calc — submit a financial calculation question
async fn calc_handler(body: ShmSlice) -> HandlerResult<VilResponse<CalcResponse>> {
    let req: CalcRequest = body.json().expect("invalid JSON body");
    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": req.prompt}
        ],
        "tools": [{
            "type": "function",
            "function": {
                "name": "calculator",
                "description": "Evaluate arithmetic and algebraic expressions",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "expression": {"type": "string", "description": "Math expression to evaluate"}
                    },
                    "required": ["expression"]
                }
            }
        }],
        "stream": true
    });

    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .json_tap("choices[0].delta.content")
        .body(body);

    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    // Note: tool execution via LLM function calling. Local eval pending.
    // The tool schema is sent to the LLM but tool calls are not executed locally.
    let content = collector
        .collect_text()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Semantic type anchors
    let _event = std::any::type_name::<AgentCompletionEvent>();
    let _fault = std::any::type_name::<AgentFault>();
    let _state = std::any::type_name::<AgentMemoryState>();

    Ok(VilResponse::ok(CalcResponse {
        content,
        tools_used: vec!["calculator".into()],
    }))
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  401 — Agent Calculator (VilApp)                           ║");
    // Banner: display pipeline topology and connection info
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: Simplest agent — single calculator tool           ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    // Display authentication mode (API key vs simulator)
    println!(
        "  Auth: {}",
        if api_key.is_empty() {
            "simulator mode"
        } else {
            "OPENAI_API_KEY"
        }
    );
    println!("  Listening on http://localhost:3120/api/calc");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();

    let svc = ServiceProcess::new("calc-agent")
        .prefix("/api")
        .endpoint(Method::POST, "/calc", post(calc_handler))
        .emits::<AgentCompletionEvent>()
        .faults::<AgentFault>()
        .manages::<AgentMemoryState>();

    // Run as VilApp — financial calculator agent service
    VilApp::new("calculator-agent")
        .port(3120)
        .service(svc)
        .run()
        .await;
}
