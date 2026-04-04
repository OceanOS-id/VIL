// 107-pipeline-process-traced — Swift SDK equivalent
// Compile: vil compile --from swift --input 107-pipeline-process-traced/main.swift --release

let p = VilPipeline(name: "SupplyChainTrackedPipeline", port: 3107)
p.sink(name: "tracking_sink", port: 3107, path: "/traced")
p.source(name: "supply_chain_source", url: "http://localhost:18081/api/v1/credits/stream", format: "sse")
p.route(from: "sink.trigger_out", to: "source.trigger_in", mode: "LoanWrite")
p.route(from: "source.tracking_data_out", to: "sink.tracking_data_in", mode: "LoanWrite")
p.route(from: "source.delivery_ctrl_out", to: "sink.delivery_ctrl_in", mode: "Copy")
p.compile()
