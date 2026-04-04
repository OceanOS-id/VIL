// 507-villog-bench-file-drain — Swift SDK equivalent
// Compile: vil compile --from swift --input 507-villog-bench-file-drain/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
