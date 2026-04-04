// 508-villog-bench-multithread — Swift SDK equivalent
// Compile: vil compile --from swift --input 508-villog-bench-multithread/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
