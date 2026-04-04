// 107-pipeline-process-traced — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 107-pipeline-process-traced/main.kt --release

fun main() {
    val p = VilPipeline("SupplyChainTrackedPipeline", 3107)
    p.sink("tracking_sink", 3107, "/traced")
    p.source("supply_chain_source", url = "http://localhost:18081/api/v1/credits/stream", format = "sse")
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
    p.route("source.tracking_data_out", "sink.tracking_data_in", "LoanWrite")
    p.route("source.delivery_ctrl_out", "sink.delivery_ctrl_in", "Copy")
    p.compile()
}
