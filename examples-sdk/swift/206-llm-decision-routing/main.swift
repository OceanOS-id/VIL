// 206-llm-decision-routing — Swift SDK equivalent
// Compile: vil compile --from swift --input 206-llm-decision-routing/main.swift --release

let server = VilServer(name: "insurance-underwriting-ai", port: 3116)
let underwriter = ServiceProcess(name: "underwriter")
server.service(underwriter)
server.compile()
