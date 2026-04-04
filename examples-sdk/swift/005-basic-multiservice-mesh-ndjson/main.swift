// 005-basic-multiservice-mesh-ndjson — Swift SDK equivalent
// Compile: vil compile --from swift --input 005-basic-multiservice-mesh-ndjson/main.swift --release

let p = VilPipeline(name: "MultiServiceMesh", port: 3084)
p.sink(name: "gateway", port: 3084, path: "/ingest")
p.source(name: "credit_ingest", format: "json")
p.route(from: "gateway.trigger_out", to: "ingest.trigger_in", mode: "LoanWrite")
p.route(from: "ingest.response_data_out", to: "gateway.response_data_in", mode: "LoanWrite")
p.route(from: "ingest.response_ctrl_out", to: "gateway.response_ctrl_in", mode: "Copy")
p.compile()
