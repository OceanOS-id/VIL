// 201-llm-basic-chat — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/201-llm-basic-chat/vwfd/workflows", 3100)
        .run()
        .await;
}
