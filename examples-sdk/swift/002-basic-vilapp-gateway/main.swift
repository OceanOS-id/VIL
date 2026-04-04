// 002-basic-vilapp-gateway — Swift SDK equivalent
// Compile: vil compile --from swift --input 002-basic-vilapp-gateway/main.swift --release

let server = VilServer(name: "vil-app-gateway", port: 3081)
let gw = ServiceProcess(name: "gw")
gw.endpoint(method: "POST", path: "/trigger", handler: "trigger_handler")
server.service(gw)
server.compile()
