// 504-villog-benchmark-comparison — Swift SDK equivalent
// Compile: vil compile --from swift --input 504-villog-benchmark-comparison/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
