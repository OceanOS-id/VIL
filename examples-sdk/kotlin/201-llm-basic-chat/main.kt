// 201-llm-basic-chat — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 201-llm-basic-chat/main.kt --release

fun main() {
    val server = VilServer("llm-basic-chat", 3100)
    val chat = ServiceProcess("chat")
    chat.endpoint("POST", "/chat", "chat_handler")
    server.service(chat)
    server.compile()
}
