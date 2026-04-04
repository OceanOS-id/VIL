// 007-basic-credit-npl-filter — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 007-basic-credit-npl-filter/main.kt --release

fun main() {
    val p = VilPipeline("NplFilterPipeline", 3081)
    p.sink("npl_filter_sink", 3081, "/filter-npl")
    p.source("npl_credit_source", format = "json")
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
    p.compile()
}
