// 302-rag-multi-source-fanin — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 302-rag-multi-source-fanin/main.kt --release

fun main() {
    val server = VilServer("rag-multi-source-fanin", 3111)
    server.compile()
}
