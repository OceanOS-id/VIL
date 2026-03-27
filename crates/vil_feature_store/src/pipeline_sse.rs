// =============================================================================
// VIL Pipeline SSE — Feature Store
// =============================================================================

use vil_sdk::prelude::*;

/// Creates an HTTP sink that accepts feature store requests.
pub fn feature_store_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("FeatureStoreSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Creates an HTTP source that streams feature store events via SSE.
pub fn feature_store_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("FeatureStoreSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("features")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "action": "stream_features",
            "stream": true
        }))
}
