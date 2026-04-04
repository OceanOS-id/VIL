// 008-basic-credit-quality-monitor — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 008-basic-credit-quality-monitor/main.kt --release

fun main() {
    val p = VilPipeline("CreditQualityMonitorPipeline", 3082)
    p.sink("quality_monitor_sink", 3082, "/quality-check")
    p.source("quality_credit_source", format = "json")
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
    p.compile()
}
