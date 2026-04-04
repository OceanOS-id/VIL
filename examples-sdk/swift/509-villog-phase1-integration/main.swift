// 509-villog-phase1-integration — Swift SDK equivalent
// Compile: vil compile --from swift --input 509-villog-phase1-integration/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
