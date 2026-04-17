// 101c — VilApp Multi-Pipeline Benchmark (VWFD)
// Business logic identical to standard:
//   - SSE collect from AI sim :4545 → buffered JSON response
//   - Same as 101b but buffered (collect_text), not streaming

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/101c-vilapp-multi-pipeline-benchmark/vwfd/workflows", 3202)
        .run().await;
}
