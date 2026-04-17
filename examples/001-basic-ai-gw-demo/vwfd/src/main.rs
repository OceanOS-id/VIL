// 001-basic-ai-gw-demo — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/001-basic-ai-gw-demo/vwfd/workflows", 3080)
        .run()
        .await;
}
