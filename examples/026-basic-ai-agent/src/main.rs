// ╔════════════════════════════════════════════════════════════╗
// ║  026 — IT Helpdesk Automation Agent                       ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   IT Operations / Helpdesk Support               ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Features: ShmSlice, VilResponse, SseCollect, Agent tools ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   Automated IT support agent with diagnostic tools. In enterprise IT,
//   helpdesk tickets for common issues (password resets, disk space,
//   network connectivity) consume significant support engineer time.
//   This agent automates first-line triage:
//
//   - Calculator tool: compute storage quotas, bandwidth calculations
//   - Search tool: look up known issues in the IT knowledge base
//   - Step-by-step reasoning: show diagnostic steps for audit trail
//
//   Business impact:
//   - Reduces mean-time-to-resolution (MTTR) from hours to minutes
//   - Frees IT engineers for complex infrastructure work
//   - Provides 24/7 helpdesk coverage without staffing costs
//   - Audit trail of all diagnostic steps for compliance
//
// Agent Pattern:
//   Unlike simple chat (024), an agent can use TOOLS to take actions.
//   The LLM decides which tool to invoke based on the user's question,
//   executes the tool, and incorporates the result into its answer.
//
// Run:
//   cargo run -p basic-usage-ai-agent
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What is 42 * 13?"}' \
//     http://localhost:3092/api/agent

use vil_server::prelude::*;

// Semantic types from vil_agent plugin — compile-time validation
// ensures the IT helpdesk agent correctly reports completions,
// faults, and memory state to the observability infrastructure.
use vil_agent::semantic::{AgentCompletionEvent, AgentFault, AgentMemoryState};

// Upstream LLM endpoint for agent reasoning. The agent sends tool
// descriptions to the LLM and lets it decide which tool to invoke.
const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── IT Helpdesk Tool descriptions ──────────────────────────────────
// These describe the diagnostic tools available to the IT helpdesk agent.
// In production, tools would include: ping, traceroute, disk-check,
// password-reset, ticket-create, and knowledge-base-search.

const TOOLS: &[&str] = &[
    "calculator — evaluate math expressions (e.g. '42 * 13')",
    "search — search the knowledge base for factual information",
];

// ── Request / Response ───────────────────────────────────────────────
// The helpdesk API: employees submit IT questions, get diagnostic answers.

// AgentRequest: an employee's IT support question. In a full helpdesk,
// this would also include employee_id, department, device_type,
// and urgency_level for prioritization.
#[derive(Debug, Deserialize)]
struct AgentRequest {
    prompt: String,
}

// AgentResponse: the agent's diagnostic answer with reasoning steps.
// VilModel enables zero-copy serialization for the response payload.
#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct AgentResponse {
    content: String,
}

// ── Handler: IT helpdesk agent with tool-use reasoning ──────────────
// This handler implements the agent loop:
// 1. Receive the employee's IT question
// 2. Build a system prompt listing available diagnostic tools
// 3. Send to LLM which decides which tool to use and shows reasoning
// 4. Collect the complete diagnostic response
// 5. Return the answer with step-by-step reasoning for audit
//
// In production, this would include actual tool execution (not just
// LLM reasoning about tools) with a ReAct loop.

async fn agent_handler(body: ShmSlice) -> HandlerResult<VilResponse<AgentResponse>> {
    let req: AgentRequest = body.json().expect("invalid JSON body");

    // Build the agent system prompt with available IT diagnostic tools.
    // The instruction to "show reasoning step by step" ensures an
    // audit trail for IT compliance and quality review.
    let system_prompt = format!(
        "You are an AI agent with access to the following tools:\n\n{}\n\n\
         When the user asks a question, decide which tool to use, execute it, \
         and provide the final answer. Show your reasoning step by step.",
        TOOLS
            .iter()
            .map(|t| format!("- {}", t))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": req.prompt}
        ],
        "stream": true
    });

    // Read API key from env (empty = simulator mode, no auth needed)
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    // json_tap for precise content extraction from the agent's
    // streaming response — captures both reasoning and final answer.
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .json_tap("choices[0].delta.content")
        .body(body);

    // Add auth if API key is set (skip for local simulator)
    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    let content = collector
        .collect_text()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Semantic audit: record the agent completion event for IT metrics.
    // In production, this feeds into dashboards tracking:
    // - Tools used per query (tool utilization rates)
    // - Iteration count (reasoning complexity indicator)
    // - Resolution time (MTTR measurement)
    // - Escalation rate (queries the agent couldn't resolve)
    let _event = AgentCompletionEvent {
        query_summary: req.prompt,
        answer_length: content.len() as u32,
        tools_used: vec![],
        iterations: 1,
        total_ms: 0,
    };

    Ok(VilResponse::ok(AgentResponse { content }))
}

// ── Main ─────────────────────────────────────────────────────────────
// Bootstrap the IT helpdesk automation agent.

#[tokio::main]
async fn main() {
    // Log semantic type registration for compile-time Tri-Lane validation.
    let _ = std::any::type_name::<AgentCompletionEvent>();
    let _ = std::any::type_name::<AgentFault>();
    let _ = std::any::type_name::<AgentMemoryState>();

    println!("======================================================================");
    println!("  Example 025: AI Agent — VilApp (Layer F)");
    println!("  Semantic: AgentCompletionEvent / AgentFault / AgentMemoryState");
    println!("  Tools: calculator, search");
    println!("======================================================================");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!(
        "  Auth: {}",
        if api_key.is_empty() {
            "simulator mode (no auth)"
        } else {
            "OPENAI_API_KEY (Bearer)"
        }
    );
    println!("  Listening on http://localhost:3092/api/agent");
    println!("  Upstream: {} (stream: true)", UPSTREAM_URL);
    println!();

    // The "agent" ServiceProcess handles all IT helpdesk automation.
    // Semantic types enable automatic tracking of agent performance,
    // tool usage, and fault rates for IT operations dashboards.
    let svc = ServiceProcess::new("agent")
        .emits::<AgentCompletionEvent>() // Data lane: completion metrics
        .faults::<AgentFault>() // Fault lane: tool/LLM failures
        .manages::<AgentMemoryState>() // Control lane: conversation memory
        .prefix("/api")
        .endpoint(Method::POST, "/agent", post(agent_handler));

    VilApp::new("ai-agent").port(3092).service(svc).run().await;
}
