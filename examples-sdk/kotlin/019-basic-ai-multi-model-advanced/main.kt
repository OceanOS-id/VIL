// 019-basic-ai-multi-model-advanced — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 019-basic-ai-multi-model-advanced/main.kt --release

fun main() {
    val p = VilPipeline("AdvancedMultiModelRouterPipeline", 3086)
    p.sink("advanced_router_sink", 3086, "/route-advanced")
    p.source("advanced_router_source", url = "http://127.0.0.1:4545/v1/chat/completions", format = "sse")
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
    p.compile()
}
