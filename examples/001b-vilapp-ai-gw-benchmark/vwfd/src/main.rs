// 001b-vilapp-ai-gw-benchmark — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/001b-vilapp-ai-gw-benchmark/vwfd/workflows", 3081)
        .run()
        .await;
}
