// 006-basic-shm-extractor — Swift SDK equivalent
// Compile: vil compile --from swift --input 006-basic-shm-extractor/main.swift --release

let server = VilServer(name: "shm-extractor-demo", port: 8080)
let shm_demo = ServiceProcess(name: "shm-demo")
shm_demo.endpoint(method: "POST", path: "/ingest", handler: "ingest")
shm_demo.endpoint(method: "POST", path: "/compute", handler: "compute")
shm_demo.endpoint(method: "GET", path: "/shm-stats", handler: "shm_stats")
shm_demo.endpoint(method: "GET", path: "/benchmark", handler: "benchmark")
server.service(shm_demo)
server.compile()
