// 302-rag-multi-source-fanin — Kotlin SDK equivalent
fun main() {
    val p = VilPipeline("rag-multi-source-fanin", 3111)
    p.compile()
}
