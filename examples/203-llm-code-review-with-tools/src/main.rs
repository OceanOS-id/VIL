// ╔════════════════════════════════════════════════════════════╗
// ║  203 — Automated Code Review Assistant                    ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Macros:   ShmSlice, ServiceCtx, VilResponse, #[vil_fault]║
// ║  Domain:   PR review bot that reads code, runs static     ║
// ║            analysis tools, provides improvement suggestions║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p llm-plugin-usage-code-assistant
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"code": "fn fib(n: u64) -> u64 { if n < 2 { n } else { fib(n-1) + fib(n-2) } }"}' \
//     http://localhost:3102/api/code/review
//
// BUSINESS CONTEXT:
//   Automated PR review bot for engineering teams. When a developer submits
//   code for review, the bot: (1) runs static analysis (unwrap detection,
//   unsafe blocks, missing docs), (2) computes cyclomatic complexity, and
//   (3) synthesizes findings into actionable review comments. This reduces
//   senior engineer review burden by catching mechanical issues automatically,
//   letting humans focus on design and architecture decisions.
//
// HOW THIS DIFFERS FROM 201:
//   201 = single prompt -> single LLM call -> return
//   203 = prompt -> LLM call -> parse for <tool>...</tool> -> execute tool
//         -> feed result back into conversation -> second LLM call -> return
//   This is a MULTI-TURN conversation with LOCAL tool execution.

use vil_server::prelude::*;
use vil_llm::semantic::{LlmResponseEvent, LlmFault, LlmUsageState};

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────
// Business metrics tracked per review session:
//   total_reviews: team-wide adoption metric (target: 80% of PRs)
//   tool_invocations: measures how often the bot uses analysis tools
//   avg_complexity: codebase health indicator (< 10 is good)

#[derive(Clone, Debug)]
pub struct CodeReviewState {
    pub total_reviews: u64,
    pub tool_invocations: u64,
    pub avg_complexity: f64,
}

#[derive(Clone, Debug)]
pub struct ToolExecutedEvent {
    pub tool_name: String,
    pub input_snippet: String,
    pub result_snippet: String,
    pub turn_number: u32,
}

#[vil_fault]
pub enum CodeReviewFault {
    ToolParseFailed,
    UnknownTool,
    CalculatorOverflow,
    MaxTurnsExceeded,
    LlmUpstreamError,
}

// ── Request / Response ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CodeReviewRequest {
    code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ToolInvocation {
    tool: String,
    input: String,
    output: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct CodeReviewResponse {
    review: String,
    tools_executed: Vec<ToolInvocation>,
    turns: u32,
}

// ── Local Tool Execution ────────────────────────────────────────────

/// Parse <tool>name:input</tool> patterns from LLM output.
/// The XML-style tags are chosen because they survive tokenization
/// better than JSON in streaming LLM responses.
fn parse_tool_calls(text: &str) -> Vec<(String, String)> {
    let mut calls = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("<tool>") {
        if let Some(end) = remaining[start..].find("</tool>") {
            let inner = &remaining[start + 6..start + end];
            if let Some(colon) = inner.find(':') {
                let name = inner[..colon].trim().to_string();
                let input = inner[colon + 1..].trim().to_string();
                calls.push((name, input));
            }
            remaining = &remaining[start + end + 7..];
        } else {
            break;
        }
    }
    calls
}

/// Execute a tool locally and return result string.
/// Business: tools run in-process (no external service calls) for speed.
/// In production, these would integrate with real linters (clippy, rustfmt).
fn execute_tool(name: &str, input: &str) -> String {
    match name {
        "calculator" => {
            // Simple expression evaluator for common patterns
            let trimmed = input.trim();
            if let Some(rest) = trimmed.strip_prefix("lines(") {
                // lines(code) -> count lines
                let code = rest.trim_end_matches(')');
                let count = code.lines().count();
                format!("{} lines", count)
            } else if let Some(rest) = trimmed.strip_prefix("complexity(") {
                // complexity(code) -> estimate cyclomatic complexity
                let code = rest.trim_end_matches(')');
                let branches = code.matches("if ").count()
                    + code.matches("match ").count()
                    + code.matches("while ").count()
                    + code.matches("for ").count()
                    + code.matches("||").count()
                    + code.matches("&&").count();
                let complexity = branches + 1;
                format!("cyclomatic_complexity = {}", complexity)
            } else {
                // Try basic arithmetic: "a + b", "a * b", etc.
                let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
                if parts.len() == 3 {
                    let a = parts[0].parse::<f64>().unwrap_or(0.0);
                    let b = parts[2].parse::<f64>().unwrap_or(0.0);
                    let result = match parts[1] {
                        "+" => a + b,
                        "-" => a - b,
                        "*" => a * b,
                        "/" => if b != 0.0 { a / b } else { f64::NAN },
                        _ => f64::NAN,
                    };
                    format!("{}", result)
                } else {
                    format!("cannot evaluate: {}", trimmed)
                }
            }
        }
        "analyzer" => {
            // Static analysis: detect patterns in code
            let mut findings = Vec::new();
            if input.contains("unwrap()") {
                findings.push("WARNING: unwrap() detected — consider using ? or expect()");
            }
            if input.contains("unsafe") {
                findings.push("WARNING: unsafe block detected — verify memory safety");
            }
            if input.contains("clone()") {
                findings.push("NOTE: clone() detected — check if borrowing would suffice");
            }
            if input.contains("panic!") || input.contains("todo!") {
                findings.push("WARNING: panic!/todo! macro — not suitable for production");
            }
            if !input.contains("///") && !input.contains("//") {
                findings.push("NOTE: no comments/docs found — consider adding documentation");
            }
            if input.lines().count() > 50 {
                findings.push("NOTE: function exceeds 50 lines — consider splitting");
            }
            if findings.is_empty() {
                "No issues found. Code looks clean.".to_string()
            } else {
                findings.join("\n")
            }
        }
        _ => format!("Unknown tool: {}", name),
    }
}

// ── System Prompt ───────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = "\
You are a code review assistant with two tools available.

To use a tool, write: <tool>TOOL_NAME:INPUT</tool>

Available tools:
- calculator: Compute metrics. Examples:
  <tool>calculator:lines(fn main() { })</tool>
  <tool>calculator:complexity(if x { } else { match y { } })</tool>
  <tool>calculator:42 + 13</tool>

- analyzer: Static analysis on code snippets. Example:
  <tool>analyzer:fn foo() { vec.unwrap(); }</tool>

WORKFLOW:
1. First, use <tool>analyzer:CODE_HERE</tool> to check the submitted code.
2. Then use <tool>calculator:complexity(CODE_HERE)</tool> to compute complexity.
3. Finally, provide your review incorporating tool results.

Always include at least one tool call in your first response.";

// ── Handler: multi-turn with tool execution ─────────────────────────

async fn code_review_handler(
    ctx: ServiceCtx, body: ShmSlice,
) -> HandlerResult<VilResponse<CodeReviewResponse>> {
    let req: CodeReviewRequest = body.json().expect("invalid JSON body");
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let mut messages = vec![
        serde_json::json!({"role": "system", "content": SYSTEM_PROMPT}),
        serde_json::json!({"role": "user", "content": format!("Review this code:\n```\n{}\n```", req.code)}),
    ];

    let mut all_tools = Vec::new();
    // Cap at 3 turns to bound latency and cost — most reviews complete in 2.
    let max_turns = 3;

    for turn in 0..max_turns {
        // Call LLM — each turn costs ~$0.03 with GPT-4 (input + output tokens)
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

        let llm_output = collector.collect_text().await
            .map_err(|e| VilError::internal(e.to_string()))?;

        // Parse for tool calls
        let tool_calls = parse_tool_calls(&llm_output);

        if tool_calls.is_empty() {
            // No more tool calls — LLM is done, return final review.
            // Business: a complete review typically includes 2+ tool calls
            // (analyzer + complexity). If the LLM skips tools, the review
            // quality may be lower — tracked via tool_invocations metric.
            return Ok(VilResponse::ok(CodeReviewResponse {
                review: llm_output,
                tools_executed: all_tools,
                turns: turn + 1,
            }));
        }

        // Execute each tool and build tool results
        let mut tool_results = Vec::new();
        for (name, input) in &tool_calls {
            let output = execute_tool(name, input);
            tool_results.push(format!("[Tool: {}]\nInput: {}\nResult: {}", name, input, output));
            all_tools.push(ToolInvocation {
                tool: name.clone(),
                input: input.clone(),
                output: output.clone(),
            });
        }

        // Add assistant response + tool results to conversation
        messages.push(serde_json::json!({"role": "assistant", "content": llm_output}));
        messages.push(serde_json::json!({
            "role": "user",
            "content": format!("Tool execution results:\n\n{}\n\nNow provide your final review incorporating these results.", tool_results.join("\n\n"))
        }));
    }

    // Max turns reached — return what we have
    Ok(VilResponse::ok(CodeReviewResponse {
        review: "Max review turns reached. Please simplify the code for analysis.".into(),
        tools_executed: all_tools,
        turns: max_turns,
    }))
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let _event = std::any::type_name::<LlmResponseEvent>();
    let _fault = std::any::type_name::<LlmFault>();
    let _state = std::any::type_name::<LlmUsageState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  203 — LLM Code Review with Tools (VilApp)                 ║");
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: Multi-turn LLM + local tool execution             ║");
    println!("║  Tools: calculator (lines/complexity/math), analyzer        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!("  Auth: {}", if api_key.is_empty() { "simulator mode" } else { "OPENAI_API_KEY" });
    println!("  Listening on http://localhost:3102/api/code/review");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();

    let svc = ServiceProcess::new("code-review")
        .prefix("/api")
        .emits::<LlmResponseEvent>()
        .faults::<LlmFault>()
        .manages::<LlmUsageState>()
        .endpoint(Method::POST, "/code/review", post(code_review_handler));

    VilApp::new("llm-code-review-tools")
        .port(3102)
        .service(svc)
        .run()
        .await;
}
