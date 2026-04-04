// 022-basic-sidecar-python — Swift SDK equivalent
// Compile: vil compile --from swift --input 022-basic-sidecar-python/main.swift --release

let server = VilServer(name: "sidecar-python-example", port: 8080)
let fraud = ServiceProcess(name: "fraud")
fraud.endpoint(method: "GET", path: "/status", handler: "fraud_status")
fraud.endpoint(method: "POST", path: "/check", handler: "fraud_check")
server.service(fraud)
let root = ServiceProcess(name: "root")
root.endpoint(method: "GET", path: "/", handler: "index")
server.service(root)
server.compile()
