// ╔════════════════════════════════════════════════════════════╗
// ║  010 — Customer Support Live Chat                         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: VilResponse                                      ║
// ║  Domain:   Real-time support agent <-> customer chat with   ║
// ║            message history, presence tracking, and stats    ║
// ╚════════════════════════════════════════════════════════════╝
// basic-usage-websocket-chat — WebSocket Echo + Broadcast Chat Room (VX Architecture)
// =============================================================================
//
// BUSINESS CONTEXT:
//   Customer support live chat for an e-commerce platform. Support agents and
//   customers connect via WebSocket for real-time bidirectional messaging.
//   The system tracks active connections (agent availability) and total message
//   count (SLA metrics). In production, this would integrate with a ticketing
//   system, agent routing queue, and conversation persistence layer.
//
// Demonstrates vil-server's WebSocket support using the VX Process-Oriented
// architecture (VilApp + ServiceProcess). Clients connect via WebSocket, send
// JSON messages, and the server broadcasts to all connected clients using
// tokio::sync::broadcast.
//
// VX highlights:
//   - ServiceProcess groups endpoints as a logical Process
//   - VilApp orchestrates processes with Tri-Lane mesh
//   - Handlers stay EXACTLY the same as classic vil-server
//
// Endpoints:
//   GET  /          — HTML page with embedded JavaScript WebSocket client
//   GET  /ws        — WebSocket upgrade handler (bidirectional chat)
//   GET  /stats     — connected client count
//
// Built-in endpoints (auto-provided by VilApp):
//   GET  /health    — health check
//   GET  /ready     — readiness probe
//   GET  /metrics   — Prometheus-style metrics
//   GET  /info      — server info
//
// Run:
//   cargo run -p basic-usage-websocket-chat
//
// Test:
//   # Open the HTML chat client in multiple browser tabs:
//   open http://localhost:8080/
//
//   # Or use websocat for CLI testing:
//   websocat ws://localhost:8080/api/chat/ws
//
//   # Check connected client count:
//   curl http://localhost:8080/api/chat/stats
//
//   # Built-in endpoints:
//   curl http://localhost:8080/health
//   curl http://localhost:8080/metrics
// =============================================================================

use vil_server::prelude::*;
use vil_server::axum::extract::ws::{WebSocket, WebSocketUpgrade, Message};
use vil_server::axum::response::Html;
use vil_server::WsHub;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// VIL WebSocket Events — typed WS messages via VilWsEvent
// ---------------------------------------------------------------------------
// Business events: each message type maps to a support workflow event.
// chat.message  = conversation content (agent replies, customer questions)
// chat.presence = agent/customer join/leave (used for availability dashboard)

/// Chat message broadcast via WebSocket.
/// In production: persisted to conversation history for audit and training.
#[derive(Clone, Debug, Serialize, Deserialize, VilWsEvent)]
#[ws_event(topic = "chat.message")]
struct ChatMessage {
    from: String,
    message: String,
    timestamp: String,
}

/// User joined notification.
#[derive(Clone, Debug, Serialize, Deserialize, VilWsEvent)]
#[ws_event(topic = "chat.presence")]
struct UserJoined {
    username: String,
}

/// User left notification.
#[derive(Clone, Debug, Serialize, Deserialize, VilWsEvent)]
#[ws_event(topic = "chat.presence")]
struct UserLeft {
    username: String,
}

// ---------------------------------------------------------------------------
// Domain models — typed response structs
// ---------------------------------------------------------------------------

/// Response struct for the /api/stats endpoint.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ChatStats {
    connected_clients: u64,
    total_messages: u64,
}

/// Global chat state — WsHub replaces raw broadcast channel.
/// connected: tracks live agents+customers for availability routing
/// total_messages: SLA metric — avg messages per session indicates resolution speed
struct ChatState {
    hub: Arc<WsHub>,
    connected: AtomicU64,
    total_messages: AtomicU64,
}

fn chat_state() -> &'static ChatState {
    static STATE: OnceLock<ChatState> = OnceLock::new();
    STATE.get_or_init(|| ChatState {
        hub: Arc::new(WsHub::new()),
        connected: AtomicU64::new(0),
        total_messages: AtomicU64::new(0),
    })
}

/// GET / — serves a simple HTML chat client with embedded JavaScript.
async fn index() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html>
<head>
    <title>VLang WebSocket Chat</title>
    <style>
        body { font-family: sans-serif; max-width: 600px; margin: 40px auto; }
        #messages { border: 1px solid #ccc; height: 300px; overflow-y: auto; padding: 10px; margin-bottom: 10px; }
        .msg { margin: 4px 0; }
        .system { color: #888; font-style: italic; }
        input, button { padding: 8px; font-size: 14px; }
        #msg { width: 70%; }
    </style>
</head>
<body>
    <h2>VLang WebSocket Chat</h2>
    <div id="messages"></div>
    <div>
        <input id="username" placeholder="Username" value="" style="width: 25%; margin-bottom: 8px;" />
    </div>
    <div>
        <input id="msg" placeholder="Type a message..." />
        <button onclick="send()">Send</button>
    </div>
    <script>
        const messages = document.getElementById('messages');
        const msgInput = document.getElementById('msg');
        const usernameInput = document.getElementById('username');

        usernameInput.value = 'user_' + Math.random().toString(36).substr(2, 5);

        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        const ws = new WebSocket(proto + '//' + location.host + '/ws');

        ws.onopen = () => addMessage('Connected to chat', 'system');
        ws.onclose = () => addMessage('Disconnected', 'system');
        ws.onerror = () => addMessage('Connection error', 'system');

        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                addMessage(data.from + ': ' + data.message, 'msg');
            } catch (e) {
                addMessage(event.data, 'msg');
            }
        };

        function send() {
            const text = msgInput.value.trim();
            if (!text) return;
            const payload = JSON.stringify({
                username: usernameInput.value || 'anonymous',
                message: text
            });
            ws.send(payload);
            msgInput.value = '';
        }

        msgInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') send();
        });

        function addMessage(text, cls) {
            const div = document.createElement('div');
            div.className = cls;
            div.textContent = text;
            messages.appendChild(div);
            messages.scrollTop = messages.scrollHeight;
        }
    </script>
</body>
</html>"#)
}

/// GET /ws — WebSocket upgrade handler.
async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

/// Handle an individual WebSocket connection.
/// Uses WsHub for topic-based broadcast and VilWsEvent for typed messages.
async fn handle_socket(socket: WebSocket) {
    let state = chat_state();
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to chat.message topic via WsHub
    let mut rx = state.hub.subscribe(ChatMessage::ws_topic());

    // Increment connected count
    state.connected.fetch_add(1, Ordering::Relaxed);

    // Broadcast join notification
    let join = UserJoined { username: "anonymous".to_string() };
    join.broadcast(&state.hub);

    // Task: forward hub messages to this WebSocket client.
    // Business: ensures customer sees agent replies in real-time (< 50ms target).
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let hub = state.hub.clone();
    let total = &state.total_messages;

    // Task: receive messages from client → parse → broadcast via WsHub
    let mut recv_task = tokio::spawn({
        let hub = hub.clone();
        let total_messages = total as *const AtomicU64 as usize; // safe: 'static
        async move {
            let total = unsafe { &*(total_messages as *const AtomicU64) };
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => {
                        // Parse client JSON: { "username": "...", "message": "..." }
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                            let username = parsed.get("username")
                                .and_then(|v| v.as_str()).unwrap_or("anonymous");
                            let message = parsed.get("message")
                                .and_then(|v| v.as_str()).unwrap_or("");

                            let timestamp = {
                                use std::time::{SystemTime, UNIX_EPOCH};
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs()
                                    .to_string()
                            };

                            // Use VilWsEvent — typed message + topic broadcast
                            let chat_msg = ChatMessage {
                                from: username.to_string(),
                                message: message.to_string(),
                                timestamp,
                            };
                            chat_msg.broadcast(&hub);
                            total.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        }
    });

    // Wait for either task to finish — graceful disconnect handles
    // browser close, network drop, or agent switching to another ticket.
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    // Broadcast leave notification
    let leave = UserLeft { username: "anonymous".to_string() };
    leave.broadcast(&state.hub);

    // Decrement connected count
    state.connected.fetch_sub(1, Ordering::Relaxed);
}

/// GET /api/stats — returns chat statistics including WsHub subscriber count.
/// Business: operations dashboard uses this to monitor support queue health.
/// High connected_clients + low total_messages = agents may be idle.
async fn stats() -> VilResponse<ChatStats> {
    let state = chat_state();
    VilResponse::ok(ChatStats {
        connected_clients: state.connected.load(Ordering::Relaxed),
        total_messages: state.total_messages.load(Ordering::Relaxed),
    })
}

#[tokio::main]
async fn main() {
    // Initialize the global chat state
    let _ = chat_state();

    // VX: Define chat service as a Process — single ServiceProcess owns
    // the entire support chat domain (UI, WebSocket, stats).
    let chat_service = ServiceProcess::new("chat")
        .endpoint(Method::GET, "/", get(index))
        .endpoint(Method::GET, "/ws", get(ws_handler))
        .endpoint(Method::GET, "/stats", get(stats));

    // VX: Run as Process-Oriented app
    VilApp::new("websocket-chat")
        .port(8080)
        .service(chat_service)
        .run()
        .await;
}
