// 023-basic-hybrid-wasm-sidecar — Swift SDK equivalent
// Compile: vil compile --from swift --input 023-basic-hybrid-wasm-sidecar/main.swift --release

let server = VilServer(name: "hybrid-pipeline", port: 8080)
let pipeline = ServiceProcess(name: "pipeline")
pipeline.endpoint(method: "GET", path: "/", handler: "index")
pipeline.endpoint(method: "POST", path: "/validate", handler: "validate_order")
pipeline.endpoint(method: "POST", path: "/price", handler: "calculate_price")
pipeline.endpoint(method: "POST", path: "/fraud", handler: "fraud_check")
pipeline.endpoint(method: "POST", path: "/order", handler: "process_order")
server.service(pipeline)
server.compile()
