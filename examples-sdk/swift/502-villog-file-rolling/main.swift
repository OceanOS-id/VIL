// 502-villog-file-rolling — Swift SDK equivalent
// Compile: vil compile --from swift --input 502-villog-file-rolling/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
