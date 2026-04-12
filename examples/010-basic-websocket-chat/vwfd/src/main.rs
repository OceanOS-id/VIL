// 010 — Customer Support Chat (VWFD)
// Sidecar: Node.js chat processor
// NativeCode: stats counter (WebSocket stats = inherently stateful)

use std::sync::atomic::{AtomicU64, Ordering};

static TOTAL_MESSAGES: AtomicU64 = AtomicU64::new(0);

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/010-basic-websocket-chat/vwfd/workflows", 8080)
        .sidecar("process_chat_message", "node examples/010-basic-websocket-chat/vwfd/sidecar/nodejs/chat_processor.js")
        .native("chat_stats", |_input| {
            let total = TOTAL_MESSAGES.load(Ordering::Relaxed);
            Ok(serde_json::json!({
                "connected_clients": 0,
                "total_messages": total,
                "rooms": [],
            }))
        })
        .run()
        .await;
}
