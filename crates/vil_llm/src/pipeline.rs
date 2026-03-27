//! SSE pipeline builders for LLM operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for use with `vil_workflow!` macro. This is the **primary** VIL integration
//! layer — all AI streaming goes through Tri-Lane SSE pipelines.
//!
//! # Example
//!
//! ```rust,ignore
//! use vil_sdk::prelude::*;
//! use vil_llm::pipeline;
//!
//! let sink = pipeline::chat_sink(3090, "/chat");
//! let source = pipeline::chat_source(
//!     "http://localhost:4545/v1/chat/completions", "gpt-4",
//! );
//!
//! let (_ir, (sink_h, source_h)) = vil_workflow! {
//!     name: "LlmChat",
//!     instances: [ sink, source ],
//!     routes: [
//!         sink.trigger_out -> source.trigger_in (LoanWrite),
//!         source.response_data_out -> sink.response_data_in (LoanWrite),
//!         source.response_ctrl_out -> sink.response_ctrl_in (Copy),
//!     ]
//! };
//! ```

use vil_sdk::prelude::*;

// ── Port name constants (consistent across all LLM pipelines) ───────

const P_TRIGGER_OUT: &str = "trigger_out";
const P_TRIGGER_IN: &str = "trigger_in";
const P_DATA_IN: &str = "response_data_in";
const P_DATA_OUT: &str = "response_data_out";
const P_CTRL_IN: &str = "response_ctrl_in";
const P_CTRL_OUT: &str = "response_ctrl_out";

const SSE_JSON_TAP: &str = "choices[0].delta.content";

// ── Chat pipeline builders ──────────────────────────────────────────

/// Build an HTTP sink that accepts chat prompts via POST.
///
/// Receives user requests on `port`/`path`, forwards them as triggers
/// to the SSE source, and streams the response back to the client.
pub fn chat_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("LlmChatSink")
        .port(port)
        .path(path)
        .out_port(P_TRIGGER_OUT)
        .in_port(P_DATA_IN)
        .ctrl_in_port(P_CTRL_IN)
}

/// Build an SSE source that streams chat completions from an LLM endpoint.
///
/// Connects to `upstream_url` (OpenAI-compatible /v1/chat/completions),
/// sends the prompt with `"stream": true`, and extracts token deltas
/// via `choices[0].delta.content`.
pub fn chat_source(upstream_url: &str, model: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("LlmChatSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap(SSE_JSON_TAP)
        .in_port(P_TRIGGER_IN)
        .out_port(P_DATA_OUT)
        .ctrl_out_port(P_CTRL_OUT)
        .post_json(serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a helpful assistant."
                },
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "stream": true
        }))
}

/// Build a chat source with a custom system prompt.
pub fn chat_source_with_system(
    upstream_url: &str,
    model: &str,
    system_prompt: &str,
) -> HttpSourceBuilder {
    HttpSourceBuilder::new("LlmChatSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap(SSE_JSON_TAP)
        .in_port(P_TRIGGER_IN)
        .out_port(P_DATA_OUT)
        .ctrl_out_port(P_CTRL_OUT)
        .post_json(serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "stream": true
        }))
}
