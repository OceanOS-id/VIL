// 404-agent-data-csv-analyst — Swift SDK equivalent
// Compile: vil compile --from swift --input 404-agent-data-csv-analyst/main.swift --release

let server = VilServer(name: "csv-analyst-agent", port: 3123)
let csv_analyst_agent = ServiceProcess(name: "csv-analyst-agent")
csv_analyst_agent.endpoint(method: "POST", path: "/csv-analyze", handler: "csv_analyze_handler")
server.service(csv_analyst_agent)
server.compile()
