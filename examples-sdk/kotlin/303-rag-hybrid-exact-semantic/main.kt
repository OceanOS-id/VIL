// 303-rag-hybrid-exact-semantic — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 303-rag-hybrid-exact-semantic/main.kt --release

fun main() {
    val server = VilServer("rag-hybrid-exact-semantic", 3112)
    val rag_hybrid = ServiceProcess("rag-hybrid")
    rag_hybrid.endpoint("POST", "/hybrid", "hybrid_handler")
    server.service(rag_hybrid)
    server.compile()
}
