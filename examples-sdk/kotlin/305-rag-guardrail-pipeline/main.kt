// 305-rag-guardrail-pipeline — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 305-rag-guardrail-pipeline/main.kt --release

fun main() {
    val server = VilServer("rag-guardrail-pipeline", 3114)
    val rag_guardrail = ServiceProcess("rag-guardrail")
    rag_guardrail.endpoint("POST", "/safe-rag", "safe_rag_handler")
    server.service(rag_guardrail)
    server.compile()
}
