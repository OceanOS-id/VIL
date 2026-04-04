// 016-basic-ai-rag-gateway — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 016-basic-ai-rag-gateway/main.kt --release

fun main() {
    val p = VilPipeline("RagPipeline", 3084)
    p.sink("rag_webhook", 3084, "/rag")
    p.source("rag_sse_inference", url = "http://127.0.0.1:4545/v1/chat/completions", format = "sse")
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
    p.compile()
}
