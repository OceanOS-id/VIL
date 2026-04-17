// 106 — SSE Standard Dialect (W3C) (VWFD)
// Business logic identical to standard:
//   - Upstream: SSE from credit-sim :18081 /credits/stream
//   - Dialect: W3C Standard (not OpenAI), done_marker: [END]
//   - Streaming response back to client

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/106-pipeline-sse-standard-dialect/vwfd/workflows", 3208)
        .run().await;
}
