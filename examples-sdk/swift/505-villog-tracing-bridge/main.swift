// 505-villog-tracing-bridge — Swift SDK equivalent
// Compile: vil compile --from swift --input 505-villog-tracing-bridge/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
