// 001b-vilapp-ai-gw-benchmark — Swift SDK equivalent
// Compile: vil compile --from swift --input 001b-vilapp-ai-gw-benchmark/main.swift --release

let server = VilServer(name: "ai-gw-bench", port: 3081)
let gw = ServiceProcess(name: "gw")
server.service(gw)
server.compile()
