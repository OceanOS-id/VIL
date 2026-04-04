// 106-pipeline-sse-standard-dialect — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 106-pipeline-sse-standard-dialect/main.kt --release

fun main() {
    val p = VilPipeline("IoTSensorPipeline", 3106)
    p.sink("io_t_dashboard_sink", 3106, "/stream")
    p.source("io_t_sensor_source", url = "http://localhost:18081/api/v1/credits/stream", format = "sse")
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
    p.route("source.sensor_data_out", "sink.sensor_data_in", "LoanWrite")
    p.route("source.batch_ctrl_out", "sink.batch_ctrl_in", "Copy")
    p.compile()
}
