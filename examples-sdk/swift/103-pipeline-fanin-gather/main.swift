// 103-pipeline-fanin-gather — Swift SDK equivalent
// Compile: vil compile --from swift --input 103-pipeline-fanin-gather/main.swift --release

let p = VilPipeline(name: "CreditGatherPipeline", port: 3093)
p.sink(name: "credit_gather_sink", port: 3093, path: "/gather")
p.source(name: "credit_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json")
p.sink(name: "inventory_gather_sink", port: 3094, path: "/inventory")
p.source(name: "inventory_source", url: "http://localhost:18092/api/v1/products")
p.route(from: "credit_sink.trigger_out", to: "credit_source.trigger_in", mode: "LoanWrite")
p.route(from: "credit_source.response_data_out", to: "credit_sink.response_data_in", mode: "LoanWrite")
p.route(from: "credit_source.response_ctrl_out", to: "credit_sink.response_ctrl_in", mode: "Copy")
p.route(from: "inventory_sink.trigger_out", to: "inventory_source.trigger_in", mode: "LoanWrite")
p.route(from: "inventory_source.response_data_out", to: "inventory_sink.response_data_in", mode: "LoanWrite")
p.route(from: "inventory_source.response_ctrl_out", to: "inventory_sink.response_ctrl_in", mode: "Copy")
p.compile()
