// 034-basic-blocking-task — Swift SDK equivalent
// Compile: vil compile --from swift --input 034-basic-blocking-task/main.swift --release

let server = VilServer(name: "credit-risk-scoring-engine", port: 8080)
let risk_engine = ServiceProcess(name: "risk-engine")
risk_engine.endpoint(method: "POST", path: "/risk/assess", handler: "assess_risk")
risk_engine.endpoint(method: "GET", path: "/risk/health", handler: "risk_health")
server.service(risk_engine)
server.compile()
