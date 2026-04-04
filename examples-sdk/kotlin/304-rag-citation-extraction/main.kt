// 304-rag-citation-extraction — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 304-rag-citation-extraction/main.kt --release

fun main() {
    val server = VilServer("rag-citation-extraction", 3113)
    val rag_citation = ServiceProcess("rag-citation")
    rag_citation.endpoint("POST", "/cited-rag", "cited_rag_handler")
    server.service(rag_citation)
    server.compile()
}
