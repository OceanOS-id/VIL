// 101b-multi-pipeline-benchmark — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 101b-multi-pipeline-benchmark/main.kt --release

fun main() {
    val p = VilPipeline("MultiPipelineBench", 3090)
    p.sink("gateway", 3090, "/trigger")
    p.source("l_l_m_upstream", url = "http://127.0.0.1:4545/v1/chat/completions")
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
    p.compile()
}
