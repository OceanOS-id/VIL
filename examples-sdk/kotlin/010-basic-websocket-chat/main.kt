// 010-basic-websocket-chat — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 010-basic-websocket-chat/main.kt --release

fun main() {
    val server = VilServer("websocket-chat", 8080)
    val chat = ServiceProcess("chat")
    chat.endpoint("GET", "/", "index")
    chat.endpoint("GET", "/ws", "ws_handler")
    chat.endpoint("GET", "/stats", "stats")
    server.service(chat)
    server.compile()
}
