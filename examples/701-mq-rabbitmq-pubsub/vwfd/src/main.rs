// 701-mq-rabbitmq-pubsub — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/701-mq-rabbitmq-pubsub/vwfd/workflows", 3252)
        .run()
        .await;
}
