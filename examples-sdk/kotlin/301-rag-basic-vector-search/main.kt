// 301-rag-basic-vector-search — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 301-rag-basic-vector-search/main.kt --release

fun main() {
    val server = VilServer("rag-basic-vector-search", 3110)
    val rag_basic = ServiceProcess("rag-basic")
    rag_basic.endpoint("POST", "/rag", "rag_handler")
    server.service(rag_basic)
    server.compile()
}
