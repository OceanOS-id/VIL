// =============================================================================
// VIL Pipeline SSE — Multi-Agent
// =============================================================================

use vil_sdk::prelude::*;

/// Creates an HTTP sink that accepts multi-agent orchestration requests.
pub fn multi_agent_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("MultiAgentSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Creates an HTTP source that streams multi-agent results via SSE.
pub fn multi_agent_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("MultiAgentSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("agent_outputs")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "query": "run agents",
            "stream": true
        }))
}
