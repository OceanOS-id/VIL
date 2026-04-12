// 704-protocol-modbus-read — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/704-protocol-modbus-read/vwfd/workflows", 3255)
        .run()
        .await;
}
