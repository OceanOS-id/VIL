// 025-basic-rag-service — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 025-basic-rag-service/main.kt --release

fun main() {
    val server = VilServer("rag-service", 3091)
    val rag = ServiceProcess("rag")
    rag.endpoint("POST", "/rag", "rag_handler")
    server.service(rag)
    server.compile()
}
