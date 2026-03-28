// ╔════════════════════════════════════════════════════════════╗
// ║  405 — Autonomous Research Agent                         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                        ║
// ║  Token:    N/A                                           ║
// ║  Unique:   ReAct LOOP — Think -> Act -> Observe ->       ║
// ║            Repeat. Multi-step stateful conversation      ║
// ║            with explicit reasoning trace. Max 5 iters.   ║
// ║            Full trace included in response.              ║
// ║  Domain:   Multi-step research: search -> fetch ->       ║
// ║            calculate -> synthesize — with full trace      ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p agent-plugin-usage-multi-tool
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What is the total value of all electronics products in stock? Search for electronics, then calculate total inventory value."}' \
//     http://localhost:3124/api/react
//
// BUSINESS CONTEXT:
//   Autonomous research agent for procurement teams. A buyer asks "What is
//   the total value of all electronics products in stock?" and the agent
//   autonomously decides to: (1) search the product catalog for electronics,
//   (2) extract prices and quantities, (3) calculate total inventory value.
//   The full reasoning trace is returned so procurement managers can audit
//   the agent's logic — critical for purchase decisions over $10k that
//   require documented justification.
//
// HOW THIS DIFFERS FROM 401-404:
//   401-404 = single LLM call with pre/post tool execution
//   405 = ITERATIVE ReAct loop:
//     1. LLM thinks and decides which tool to use
//     2. Tool is executed locally
//     3. Result is fed back to LLM as observation
//     4. LLM thinks again, may use another tool
//     5. Repeats until LLM outputs FINAL_ANSWER or max 5 iterations
//   This is fundamentally different: multi-turn, stateful, self-directed.

use vil_agent::semantic::{AgentCompletionEvent, AgentFault, AgentMemoryState};
use vil_server::prelude::*;

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";
const MAX_ITERATIONS: usize = 5;

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ReactAgentState {
    pub total_queries: u64,
    pub total_iterations: u64,
    pub avg_iterations_per_query: f64,
    pub max_iter_hits: u64,
}

#[derive(Clone, Debug)]
pub struct ReactStepEvent {
    pub iteration: u32,
    pub thought: String,
    pub action: String,
    pub observation_preview: String,
}

#[vil_fault]
pub enum ReactAgentFault {
    MaxIterationsReached,
    ToolExecutionFailed,
    MalformedAction,
    LlmUpstreamError,
    LoopDetected,
}

// ── ReAct Trace Types ───────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ReactStep {
    iteration: u32,
    thought: String,
    action: Option<String>,
    action_input: Option<String>,
    observation: Option<String>,
}

// ── Available Tools ─────────────────────────────────────────────────

/// Mock knowledge base for search tool.
/// Business: simulates the procurement catalog — products, prices, stock levels.
/// In production, this would query a real inventory management system.
fn tool_search(query: &str) -> String {
    let q = query.to_lowercase();
    let mut results = Vec::new();

    if q.contains("electronics") || q.contains("product") {
        results.push(
            "Products in Electronics category: Wireless Keyboard ($49.99, 150 in stock), \
                       USB-C Hub ($29.99, 200 in stock), Ergonomic Mouse ($79.99, 80 in stock), \
                       Webcam HD ($69.99, 110 in stock).",
        );
    }
    if q.contains("furniture") {
        results.push(
            "Products in Furniture category: Standing Desk ($499.99, 25 in stock), \
                       Monitor Arm ($89.99, 60 in stock).",
        );
    }
    if q.contains("audio") || q.contains("headphone") {
        results.push("Products in Audio category: Noise-Cancel Headphones ($199.99, 45 in stock).");
    }
    if q.contains("price") || q.contains("expensive") || q.contains("cheap") {
        results.push("Price range: cheapest is USB-C Hub at $29.99, most expensive is Standing Desk at $499.99. \
                       Average price across all products: $131.87.");
    }
    if q.contains("stock") || q.contains("inventory") {
        results.push(
            "Total inventory: 970 units. Desk Lamp has highest stock (300), \
                       Standing Desk has lowest (25).",
        );
    }
    if q.contains("revenue") || q.contains("sales") || q.contains("best seller") {
        results.push(
            "Best selling categories by volume: Lighting (300 units), Electronics (540 units). \
                       By revenue estimate: Furniture leads due to Standing Desk price.",
        );
    }

    if results.is_empty() {
        format!(
            "No specific data found for query: '{}'. Try searching for product categories \
                 (electronics, furniture, audio) or metrics (price, stock, revenue).",
            query
        )
    } else {
        results.join("\n")
    }
}

/// Calculator tool
fn tool_calculator(expr: &str) -> String {
    let trimmed = expr.trim();

    // Handle sum(...) pattern
    if let Some(inner) = trimmed
        .strip_prefix("sum(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let values: Vec<f64> = inner
            .split(',')
            .filter_map(|v| v.trim().parse::<f64>().ok())
            .collect();
        let total: f64 = values.iter().sum();
        return format!("{}", (total * 100.0).round() / 100.0);
    }

    // Handle product value patterns like "49.99 * 150 + 29.99 * 200"
    // Simple tokenized evaluator for + and * chains
    let mut total = 0.0f64;
    let mut terms: Vec<&str> = trimmed.split('+').collect();
    for term in &terms {
        let factors: Vec<f64> = term
            .split('*')
            .filter_map(|f| f.trim().parse::<f64>().ok())
            .collect();
        if factors.len() >= 2 {
            total += factors.iter().product::<f64>();
        } else if factors.len() == 1 {
            total += factors[0];
        }
    }

    if total != 0.0 {
        return format!("{}", (total * 100.0).round() / 100.0);
    }

    // Simple "a OP b" fallback
    let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
    if parts.len() == 3 {
        let a = parts[0].parse::<f64>().unwrap_or(0.0);
        let b = parts[2].parse::<f64>().unwrap_or(0.0);
        let result = match parts[1] {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            "/" => {
                if b != 0.0 {
                    a / b
                } else {
                    f64::NAN
                }
            }
            _ => f64::NAN,
        };
        format!("{}", (result * 100.0).round() / 100.0)
    } else {
        format!("Cannot evaluate: {}", trimmed)
    }
}

/// Execute a tool by name
fn execute_tool(name: &str, input: &str) -> String {
    match name.trim() {
        "search" => tool_search(input),
        "calculator" => tool_calculator(input),
        _ => format!(
            "Unknown tool: {}. Available tools: search, calculator",
            name
        ),
    }
}

// ── ReAct Parser ────────────────────────────────────────────────────

/// Parse LLM output for ReAct patterns:
///   Thought: I need to...
///   Action: tool_name
///   Action Input: input_string
///   FINAL_ANSWER: the conclusion
fn parse_react_output(
    text: &str,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let mut thought = None;
    let mut action = None;
    let mut action_input = None;
    let mut final_answer = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(t) = trimmed.strip_prefix("Thought:") {
            thought = Some(t.trim().to_string());
        } else if let Some(a) = trimmed.strip_prefix("Action:") {
            action = Some(a.trim().to_string());
        } else if let Some(ai) = trimmed.strip_prefix("Action Input:") {
            action_input = Some(ai.trim().to_string());
        } else if let Some(fa) = trimmed.strip_prefix("FINAL_ANSWER:") {
            final_answer = Some(fa.trim().to_string());
        }
    }

    (thought, action, action_input, final_answer)
}

// ── Request / Response ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ReactRequest {
    prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct ReactResponse {
    answer: String,
    reasoning_trace: Vec<ReactStep>,
    iterations: u32,
    max_iter_reached: bool,
}

// ── ReAct System Prompt ─────────────────────────────────────────────

const REACT_SYSTEM_PROMPT: &str = "\
You are a reasoning agent that follows the ReAct pattern (Reason + Act).

Available tools:
- search: Look up product data, prices, inventory, and categories.
  Example: Action: search / Action Input: electronics products and prices
- calculator: Compute arithmetic expressions.
  Example: Action: calculator / Action Input: 49.99 * 150 + 29.99 * 200

FORMAT YOUR RESPONSE EXACTLY LIKE THIS:

Thought: [your reasoning about what to do next]
Action: [tool name]
Action Input: [input for the tool]

After receiving observations, continue reasoning:

Thought: [what you learned, what to do next]
Action: [next tool]
Action Input: [input]

When you have enough information, conclude with:

Thought: [final reasoning]
FINAL_ANSWER: [your complete answer with specific numbers and citations]

RULES:
- Always start with a Thought
- Use exactly one Action per response
- Wait for the Observation before continuing
- Include specific numbers in your FINAL_ANSWER
- Maximum 5 iterations allowed";

// ── Handler: ReAct Loop ─────────────────────────────────────────────

async fn react_handler(body: ShmSlice) -> HandlerResult<VilResponse<ReactResponse>> {
    let req: ReactRequest = body.json().expect("invalid JSON body");
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let mut messages = vec![
        serde_json::json!({"role": "system", "content": REACT_SYSTEM_PROMPT}),
        serde_json::json!({"role": "user", "content": req.prompt}),
    ];

    let mut trace = Vec::new();

    // ReAct loop: the agent decides its own workflow at each step.
    // This is fundamentally different from pre-scripted pipelines —
    // the LLM chooses which tool to use based on accumulated observations.
    for iteration in 0..MAX_ITERATIONS {
        // Call LLM — each iteration adds to the conversation context
        let body = serde_json::json!({
            "model": "gpt-4",
            "messages": messages,
            "stream": true
        });

        let mut collector = SseCollect::post_to(UPSTREAM_URL)
            .dialect(SseDialect::openai())
            .body(body);

        if !api_key.is_empty() {
            collector = collector.bearer_token(&api_key);
        }

        let llm_output = collector
            .collect_text()
            .await
            .map_err(|e| VilError::internal(e.to_string()))?;

        // Parse ReAct output
        let (thought, action, action_input, final_answer) = parse_react_output(&llm_output);

        // Check for FINAL_ANSWER
        if let Some(answer) = final_answer {
            trace.push(ReactStep {
                iteration: iteration as u32 + 1,
                thought: thought.unwrap_or_default(),
                action: None,
                action_input: None,
                observation: Some(format!("FINAL_ANSWER reached")),
            });

            return Ok(VilResponse::ok(ReactResponse {
                answer,
                reasoning_trace: trace,
                iterations: iteration as u32 + 1,
                max_iter_reached: false,
            }));
        }

        // Execute tool if action specified
        if let (Some(act), Some(act_input)) = (&action, &action_input) {
            let observation = execute_tool(act, act_input);

            trace.push(ReactStep {
                iteration: iteration as u32 + 1,
                thought: thought.unwrap_or_default(),
                action: Some(act.clone()),
                action_input: Some(act_input.clone()),
                observation: Some(observation.clone()),
            });

            // Add to conversation
            messages.push(serde_json::json!({"role": "assistant", "content": llm_output}));
            messages.push(serde_json::json!({
                "role": "user",
                "content": format!("Observation: {}\n\nContinue reasoning. If you have enough information, output FINAL_ANSWER: [answer]", observation)
            }));
        } else {
            // No action found — treat entire output as final answer
            trace.push(ReactStep {
                iteration: iteration as u32 + 1,
                thought: thought.unwrap_or_default(),
                action: None,
                action_input: None,
                observation: None,
            });

            return Ok(VilResponse::ok(ReactResponse {
                answer: llm_output,
                reasoning_trace: trace,
                iterations: iteration as u32 + 1,
                max_iter_reached: false,
            }));
        }
    }

    // Max iterations reached — safety valve to prevent runaway costs.
    // Business: at $0.03/iteration with GPT-4, 5 iterations = $0.15 max per query.
    let last_thought = trace
        .last()
        .and_then(|s| s.observation.clone())
        .unwrap_or_else(|| "Max iterations reached without final answer.".into());

    Ok(VilResponse::ok(ReactResponse {
        answer: format!(
            "Reached maximum {} iterations. Last observation: {}",
            MAX_ITERATIONS, last_thought
        ),
        reasoning_trace: trace,
        iterations: MAX_ITERATIONS as u32,
        max_iter_reached: true,
    }))
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  405 — Agent ReAct Multi-Tool (VilApp)                     ║");
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: ReAct loop — Think -> Act -> Observe -> Repeat    ║");
    println!("║          Multi-step stateful reasoning with trace output   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  ReAct Pattern:");
    println!("    1. LLM thinks and decides tool");
    println!("    2. Tool executes locally");
    println!("    3. Observation fed back to LLM");
    println!(
        "    4. Repeat until FINAL_ANSWER (max {} iters)",
        MAX_ITERATIONS
    );
    println!();
    println!("  Tools:");
    println!("    - search    : product data, prices, inventory");
    println!("    - calculator: arithmetic expressions");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!(
        "  Auth: {}",
        if api_key.is_empty() {
            "simulator mode"
        } else {
            "OPENAI_API_KEY"
        }
    );
    println!("  Listening on http://localhost:3124/api/react");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();

    let svc = ServiceProcess::new("react-agent")
        .prefix("/api")
        .endpoint(Method::POST, "/react", post(react_handler))
        .emits::<AgentCompletionEvent>()
        .faults::<AgentFault>()
        .manages::<AgentMemoryState>();

    VilApp::new("react-multi-tool-agent")
        .port(3124)
        .service(svc)
        .run()
        .await;
}
