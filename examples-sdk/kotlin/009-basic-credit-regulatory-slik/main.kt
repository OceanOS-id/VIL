// 009-basic-credit-regulatory-slik — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 009-basic-credit-regulatory-slik/main.kt --release

fun main() {
    val p = VilPipeline("RegulatoryStreamPipeline", 3083)
    p.sink("regulatory_sink", 3083, "/regulatory-stream")
    p.source("regulatory_source", url = "http://localhost:18081/api/v1/credits/ndjson?count=1000", format = "json")
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
    p.compile()
}
