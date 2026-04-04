// 102-pipeline-fanout-scatter — Swift SDK equivalent
// Compile: vil compile --from swift --input 102-pipeline-fanout-scatter/main.swift --release

let p = VilPipeline(name: "NplPipeline", port: 3091)
p.sink(name: "npl_sink", port: 3091, path: "/npl")
p.source(name: "npl_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json")
p.sink(name: "healthy_sink", port: 3092, path: "/healthy")
p.source(name: "healthy_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json")
p.route(from: "npl_sink.trigger_out", to: "npl_source.trigger_in", mode: "LoanWrite")
p.route(from: "npl_source.response_data_out", to: "npl_sink.response_data_in", mode: "LoanWrite")
p.route(from: "npl_source.response_ctrl_out", to: "npl_sink.response_ctrl_in", mode: "Copy")
p.route(from: "healthy_sink.trigger_out", to: "healthy_source.trigger_in", mode: "LoanWrite")
p.route(from: "healthy_source.response_data_out", to: "healthy_sink.response_data_in", mode: "LoanWrite")
p.route(from: "healthy_source.response_ctrl_out", to: "healthy_sink.response_ctrl_in", mode: "Copy")
p.compile()
