// 027-basic-vilserver-minimal — Swift SDK equivalent
// Compile: vil compile --from swift --input 027-basic-vilserver-minimal/main.swift --release

let server = VilServer(name: "app", port: 8080)
server.compile()
