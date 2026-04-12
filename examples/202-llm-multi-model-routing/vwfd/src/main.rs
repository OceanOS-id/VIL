// 202-llm-multi-model-routing — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/202-llm-multi-model-routing/vwfd/workflows", 8080)
        .run()
        .await;
}
