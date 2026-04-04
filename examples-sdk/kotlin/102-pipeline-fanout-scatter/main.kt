// 102-pipeline-fanout-scatter — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 102-pipeline-fanout-scatter/main.kt --release

fun main() {
    val p = VilPipeline("NplPipeline", 3091)
    p.sink("npl_sink", 3091, "/npl")
    p.source("npl_source", url = "http://localhost:18081/api/v1/credits/ndjson?count=100", format = "json")
    p.sink("healthy_sink", 3092, "/healthy")
    p.source("healthy_source", url = "http://localhost:18081/api/v1/credits/ndjson?count=100", format = "json")
    p.route("npl_sink.trigger_out", "npl_source.trigger_in", "LoanWrite")
    p.route("npl_source.response_data_out", "npl_sink.response_data_in", "LoanWrite")
    p.route("npl_source.response_ctrl_out", "npl_sink.response_ctrl_in", "Copy")
    p.route("healthy_sink.trigger_out", "healthy_source.trigger_in", "LoanWrite")
    p.route("healthy_source.response_data_out", "healthy_sink.response_data_in", "LoanWrite")
    p.route("healthy_source.response_ctrl_out", "healthy_sink.response_ctrl_in", "Copy")
    p.compile()
}
