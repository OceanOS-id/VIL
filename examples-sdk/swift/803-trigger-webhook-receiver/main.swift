// 803-trigger-webhook-receiver — Swift SDK equivalent
// Compile: vil compile --from swift --input 803-trigger-webhook-receiver/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
