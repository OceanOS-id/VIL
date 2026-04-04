// 024-basic-llm-chat — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 024-basic-llm-chat/main.kt --release

fun main() {
    val server = VilServer("llm-chat", 8080)
    val chat = ServiceProcess("chat")
    chat.endpoint("POST", "/chat", "chat_handler")
    server.service(chat)
    server.compile()
}
