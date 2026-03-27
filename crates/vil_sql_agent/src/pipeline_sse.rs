//! SSE pipeline builders for SQL agent operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for SQL generation streaming via `vil_workflow!`.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts SQL generation requests via POST.
pub fn sql_agent_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("SqlAgentSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for SQL generation streaming.
pub fn sql_agent_source(upstream_url: &str, model: &str, schema_text: &str) -> HttpSourceBuilder {
    let system = format!(
        "You are a SQL query generator. Generate safe SQL queries based on the schema below.\n\nSchema:\n{}",
        schema_text
    );

    HttpSourceBuilder::new("SqlAgentSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("choices[0].delta.content")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": "Generate SQL based on the schema." }
            ],
            "stream": true
        }))
}
