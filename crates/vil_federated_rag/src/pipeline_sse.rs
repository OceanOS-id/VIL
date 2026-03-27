use vil_sdk::prelude::*;

pub fn federated_rag_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("FederatedRagSink")
        .port(port).path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn federated_rag_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("FederatedRagSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("results")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "query": "",
            "top_k": 10
        }))
}
