// 005-basic-multiservice-mesh-ndjson — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 005-basic-multiservice-mesh-ndjson/main.kt --release

fun main() {
    val p = VilPipeline("MultiServiceMesh", 3084)
    p.sink("gateway", 3084, "/ingest")
    p.source("credit_ingest", format = "json")
    p.route("gateway.trigger_out", "ingest.trigger_in", "LoanWrite")
    p.route("ingest.response_data_out", "gateway.response_data_in", "LoanWrite")
    p.route("ingest.response_ctrl_out", "gateway.response_ctrl_in", "Copy")
    p.compile()
}
