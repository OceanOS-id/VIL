// 001-basic-ai-gw-demo — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 001-basic-ai-gw-demo/main.kt --release

fun main() {
    val p = VilPipeline("DecomposedPipeline", 3080)
    p.sink("webhook_trigger", 3080, "/trigger")
    p.source("sse_inference", url = "http://127.0.0.1:4545/v1/chat/completions", format = "sse")
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
    p.compile()
}
