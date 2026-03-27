// =============================================================================
// VIL Pipeline SSE — Model Serving
// =============================================================================

use vil_sdk::prelude::*;

/// Creates an HTTP sink that accepts model inference requests.
pub fn model_serving_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("ModelServingSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Creates an HTTP source that streams inference results via SSE.
pub fn model_serving_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("ModelServingSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("choices[0].delta.content")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "infer"}],
            "stream": true
        }))
}
