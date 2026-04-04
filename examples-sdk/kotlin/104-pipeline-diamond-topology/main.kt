// 104-pipeline-diamond-topology — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 104-pipeline-diamond-topology/main.kt --release

fun main() {
    val p = VilPipeline("DiamondSummary", 3095)
    p.sink("summary_sink", 3095, "/diamond")
    p.source("summary_source", url = "http://localhost:18081/api/v1/credits/ndjson?count=100", format = "json")
    p.sink("detail_sink", 3096, "/diamond-detail")
    p.source("detail_source", url = "http://localhost:18081/api/v1/credits/ndjson?count=100", format = "json")
    p.route("summary_sink.trigger_out", "summary_source.trigger_in", "LoanWrite")
    p.route("summary_source.response_data_out", "summary_sink.response_data_in", "LoanWrite")
    p.route("summary_source.response_ctrl_out", "summary_sink.response_ctrl_in", "Copy")
    p.route("detail_sink.trigger_out", "detail_source.trigger_in", "LoanWrite")
    p.route("detail_source.response_data_out", "detail_sink.response_data_in", "LoanWrite")
    p.route("detail_source.response_ctrl_out", "detail_sink.response_ctrl_in", "Copy")
    p.compile()
}
