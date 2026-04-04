// 003-basic-hello-server — Swift SDK equivalent
// Compile: vil compile --from swift --input 003-basic-hello-server/main.swift --release

let server = VilServer(name: "vil-basic-hello-server", port: 8080)
let gw = ServiceProcess(name: "gw")
gw.endpoint(method: "POST", path: "/transform", handler: "transform")
gw.endpoint(method: "POST", path: "/echo", handler: "echo")
gw.endpoint(method: "GET", path: "/health", handler: "health")
server.service(gw)
server.compile()
