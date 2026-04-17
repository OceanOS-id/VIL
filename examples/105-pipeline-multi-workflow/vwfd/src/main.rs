// 105 — Financial Data Hub: Multi-Workflow (VWFD)
// Business logic identical to standard:
//   - AI workflow: SSE proxy to AI sim :4545
//   - Credit workflow: NDJSON stream from credit-sim :18081
//   - Inventory workflow: REST call (mocked — no inventory simulator)
// Standard uses 3 ports (3097/3098/3099). VWFD uses single port with 3 paths.

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/105-pipeline-multi-workflow/vwfd/workflows", 3207)
        .run().await;
}
