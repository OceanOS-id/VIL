// 106-pipeline-sse-standard-dialect — Zig SDK equivalent
// Compile: vil compile --from zig --input 106-pipeline-sse-standard-dialect/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("IoTSensorPipeline", 3106);
    p.sink("io_t_dashboard_sink", 3106, "/stream");
    p.source("io_t_sensor_source", "http://localhost:18081/api/v1/credits/stream", "sse");
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
    p.route("source.sensor_data_out", "sink.sensor_data_in", "LoanWrite");
    p.route("source.batch_ctrl_out", "sink.batch_ctrl_in", "Copy");
    p.compile();
}
