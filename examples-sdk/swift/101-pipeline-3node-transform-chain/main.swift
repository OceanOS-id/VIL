// 101-pipeline-3node-transform-chain — Swift SDK equivalent
// Compile: vil compile --from swift --input 101-pipeline-3node-transform-chain/main.swift --release

let p = VilPipeline(name: "TransformChainPipeline", port: 3090)
p.sink(name: "transform_gateway", port: 3090, path: "/transform")
p.source(name: "chained_transform_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json")
p.route(from: "sink.trigger_out", to: "source.trigger_in", mode: "LoanWrite")
p.route(from: "source.response_data_out", to: "sink.response_data_in", mode: "LoanWrite")
p.route(from: "source.response_ctrl_out", to: "sink.response_ctrl_in", mode: "Copy")
p.compile()
