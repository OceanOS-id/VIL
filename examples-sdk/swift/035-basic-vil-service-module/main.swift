// 035-basic-vil-service-module — Swift SDK equivalent
// Compile: vil compile --from swift --input 035-basic-vil-service-module/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
