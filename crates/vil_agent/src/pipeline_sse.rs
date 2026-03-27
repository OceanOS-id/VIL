//! SSE pipeline builders for Agent operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for agent streaming via `vil_workflow!`.
//!
//! # Example
//!
//! ```rust,ignore
//! use vil_sdk::prelude::*;
//! use vil_agent::pipeline_sse;
//!
//! let sink = pipeline_sse::agent_sink(3092, "/agent");
//! let source = pipeline_sse::agent_source(
//!     "http://localhost:4545/v1/chat/completions", "gpt-4",
//!     &["calculator", "search"],
//! );
//!
//! let (_ir, (sink_h, source_h)) = vil_workflow! {
//!     name: "AgentPipeline",
//!     instances: [ sink, source ],
//!     routes: [
//!         sink.trigger_out -> source.trigger_in (LoanWrite),
//!         source.response_data_out -> sink.response_data_in (LoanWrite),
//!         source.response_ctrl_out -> sink.response_ctrl_in (Copy),
//!     ]
//! };
//! ```

use vil_sdk::prelude::*;

const SSE_JSON_TAP: &str = "choices[0].delta.content";

/// Build an HTTP sink that accepts agent queries via POST.
pub fn agent_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("AgentSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for agent streaming with tool-calling prompt.
///
/// The system prompt equips the AI with tool descriptions so it can
/// reason about which tool to use (ReAct pattern via prompt).
pub fn agent_source(upstream_url: &str, model: &str, tool_names: &[&str]) -> HttpSourceBuilder {
    let tools_desc = tool_names
        .iter()
        .map(|t| format!("- {}", t))
        .collect::<Vec<_>>()
        .join("\n");

    let system = format!(
        "You are an AI agent with access to tools. When answering questions:\n\
         1. Think step-by-step about what tools you need\n\
         2. Use [TOOL:name] input [/TOOL] to call a tool\n\
         3. Show your reasoning, then give the final answer\n\n\
         Available tools:\n{}",
        tools_desc
    );

    HttpSourceBuilder::new("AgentSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap(SSE_JSON_TAP)
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": "Hello" }
            ],
            "stream": true
        }))
}
