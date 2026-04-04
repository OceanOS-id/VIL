// 306-rag-ai-event-tracking — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 306-rag-ai-event-tracking/main.kt --release

fun main() {
    val server = VilServer("customer-support-rag", 3116)
    val support = ServiceProcess("support")
    support.endpoint("POST", "/support/ask", "answer_question")
    server.service(support)
    server.compile()
}
