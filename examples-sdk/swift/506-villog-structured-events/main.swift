// 506-villog-structured-events — Swift SDK equivalent
// Compile: vil compile --from swift --input 506-villog-structured-events/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
