// 026-basic-ai-agent — Swift SDK equivalent
// Compile: vil compile --from swift --input 026-basic-ai-agent/main.swift --release

let server = VilServer(name: "ai-agent", port: 8080)
let agent = ServiceProcess(name: "agent")
agent.endpoint(method: "POST", path: "/agent", handler: "agent_handler")
server.service(agent)
server.compile()
