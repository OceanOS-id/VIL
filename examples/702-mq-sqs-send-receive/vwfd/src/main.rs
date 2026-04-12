// 702-mq-sqs-send-receive — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/702-mq-sqs-send-receive/vwfd/workflows", 3253)
        .run()
        .await;
}
