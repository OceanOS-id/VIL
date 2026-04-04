// 401-agent-calculator — Swift SDK equivalent
// Compile: vil compile --from swift --input 401-agent-calculator/main.swift --release

let server = VilServer(name: "calculator-agent", port: 3120)
let calc_agent = ServiceProcess(name: "calc-agent")
calc_agent.endpoint(method: "POST", path: "/calc", handler: "calc_handler")
server.service(calc_agent)
server.compile()
