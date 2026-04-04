// 405-agent-react-multi-tool — Swift SDK equivalent
// Compile: vil compile --from swift --input 405-agent-react-multi-tool/main.swift --release

let server = VilServer(name: "react-multi-tool-agent", port: 3124)
let react_agent = ServiceProcess(name: "react-agent")
react_agent.endpoint(method: "POST", path: "/react", handler: "react_handler")
server.service(react_agent)
server.compile()
