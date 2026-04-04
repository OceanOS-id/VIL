// 205-llm-chunked-summarizer — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 205-llm-chunked-summarizer/main.kt --release

fun main() {
    val server = VilServer("ChunkedSummarizerPipeline", 8080)
    server.compile()
}
