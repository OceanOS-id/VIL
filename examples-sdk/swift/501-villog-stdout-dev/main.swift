// 501-villog-stdout-dev — Swift SDK equivalent
// Compile: vil compile --from swift --input 501-villog-stdout-dev/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
