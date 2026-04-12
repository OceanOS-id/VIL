// 206-llm-decision-routing — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/206-llm-decision-routing/vwfd/workflows", 8080)
        .run()
        .await;
}
