// 602-db-mongo-crud — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/602-db-mongo-crud/vwfd/workflows", 3242)
        .run()
        .await;
}
