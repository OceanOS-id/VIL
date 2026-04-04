// 009-basic-credit-regulatory-slik — Swift SDK equivalent
// Compile: vil compile --from swift --input 009-basic-credit-regulatory-slik/main.swift --release

let p = VilPipeline(name: "RegulatoryStreamPipeline", port: 3083)
p.sink(name: "regulatory_sink", port: 3083, path: "/regulatory-stream")
p.source(name: "regulatory_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=1000", format: "json")
p.route(from: "sink.trigger_out", to: "source.trigger_in", mode: "LoanWrite")
p.route(from: "source.response_data_out", to: "sink.response_data_in", mode: "LoanWrite")
p.route(from: "source.response_ctrl_out", to: "sink.response_ctrl_in", mode: "Copy")
p.compile()
