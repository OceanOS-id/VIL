// 402-agent-http-researcher — Swift SDK equivalent
// Compile: vil compile --from swift --input 402-agent-http-researcher/main.swift --release

let server = VilServer(name: "http-researcher-agent", port: 3121)
let research_agent = ServiceProcess(name: "research-agent")
research_agent.endpoint(method: "POST", path: "/research", handler: "research_handler")
research_agent.endpoint(method: "GET", path: "/products", handler: "products_handler")
server.service(research_agent)
server.compile()
