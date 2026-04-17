// 101b — Multi-Pipeline Benchmark (VWFD)
// Business logic identical to standard:
//   - SSE proxy to AI sim :4545 with hardcoded body
//   - json_tap: choices[0].delta.content
//   - Streaming response back to client
// Note: standard uses ShmToken (zero-copy), VWFD uses Connector

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/101b-multi-pipeline-benchmark/vwfd/workflows", 3201)
        .run().await;
}
