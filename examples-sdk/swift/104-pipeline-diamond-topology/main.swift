// 104-pipeline-diamond-topology — Swift SDK equivalent
// Compile: vil compile --from swift --input 104-pipeline-diamond-topology/main.swift --release

let p = VilPipeline(name: "DiamondSummary", port: 3095)
p.sink(name: "summary_sink", port: 3095, path: "/diamond")
p.source(name: "summary_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json")
p.sink(name: "detail_sink", port: 3096, path: "/diamond-detail")
p.source(name: "detail_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json")
p.route(from: "summary_sink.trigger_out", to: "summary_source.trigger_in", mode: "LoanWrite")
p.route(from: "summary_source.response_data_out", to: "summary_sink.response_data_in", mode: "LoanWrite")
p.route(from: "summary_source.response_ctrl_out", to: "summary_sink.response_ctrl_in", mode: "Copy")
p.route(from: "detail_sink.trigger_out", to: "detail_source.trigger_in", mode: "LoanWrite")
p.route(from: "detail_source.response_data_out", to: "detail_sink.response_data_in", mode: "LoanWrite")
p.route(from: "detail_source.response_ctrl_out", to: "detail_sink.response_ctrl_in", mode: "Copy")
p.compile()
