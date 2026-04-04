// 106-pipeline-sse-standard-dialect — Swift SDK equivalent
// Compile: vil compile --from swift --input 106-pipeline-sse-standard-dialect/main.swift --release

let p = VilPipeline(name: "IoTSensorPipeline", port: 3106)
p.sink(name: "io_t_dashboard_sink", port: 3106, path: "/stream")
p.source(name: "io_t_sensor_source", url: "http://localhost:18081/api/v1/credits/stream", format: "sse")
p.route(from: "sink.trigger_out", to: "source.trigger_in", mode: "LoanWrite")
p.route(from: "source.sensor_data_out", to: "sink.sensor_data_in", mode: "LoanWrite")
p.route(from: "source.batch_ctrl_out", to: "sink.batch_ctrl_in", mode: "Copy")
p.compile()
