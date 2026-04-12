// 706-mq-pulsar-messaging — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/706-mq-pulsar-messaging/vwfd/workflows", 8080)
        .run()
        .await;
}
