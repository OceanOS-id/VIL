// 038-basic-vil-app-dsl — Swift SDK equivalent
// Compile: vil compile --from swift --input 038-basic-vil-app-dsl/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
