// 406-agent-vil-handler-shm — Swift SDK equivalent
// Compile: vil compile --from swift --input 406-agent-vil-handler-shm/main.swift --release

let server = VilServer(name: "fraud-detection-agent", port: 3126)
let fraud_agent = ServiceProcess(name: "fraud-agent")
fraud_agent.endpoint(method: "POST", path: "/detect", handler: "detect_fraud")
fraud_agent.endpoint(method: "GET", path: "/health", handler: "health")
server.service(fraud_agent)
server.compile()
