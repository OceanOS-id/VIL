// 503-villog-multi-drain — Swift SDK equivalent
// Compile: vil compile --from swift --input 503-villog-multi-drain/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
