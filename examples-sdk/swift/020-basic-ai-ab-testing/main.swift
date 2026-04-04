// 020-basic-ai-ab-testing — Swift SDK equivalent
// Compile: vil compile --from swift --input 020-basic-ai-ab-testing/main.swift --release

let server = VilServer(name: "ai-ab-testing-gateway", port: 8080)
let ab = ServiceProcess(name: "ab")
ab.endpoint(method: "POST", path: "/infer", handler: "infer")
ab.endpoint(method: "GET", path: "/metrics", handler: "metrics")
ab.endpoint(method: "POST", path: "/config", handler: "update_config")
server.service(ab)
let root = ServiceProcess(name: "root")
root.endpoint(method: "GET", path: "/", handler: "index")
server.service(root)
server.compile()
