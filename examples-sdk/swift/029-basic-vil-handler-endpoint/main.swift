// 029-basic-vil-handler-endpoint — Swift SDK equivalent
// Compile: vil compile --from swift --input 029-basic-vil-handler-endpoint/main.swift --release

let server = VilServer(name: "macro-demo", port: 8080)
let demo = ServiceProcess(name: "demo")
demo.endpoint(method: "GET", path: "/plain", handler: "plain_handler")
demo.endpoint(method: "GET", path: "/handled", handler: "handled_handler")
demo.endpoint(method: "POST", path: "/endpoint", handler: "endpoint_handler")
server.service(demo)
server.compile()
