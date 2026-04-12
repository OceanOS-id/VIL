// 204-llm-streaming-translator — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/204-llm-streaming-translator/vwfd/workflows", 3103)
        .run()
        .await;
}
