// 039-basic-observer-dashboard — Swift SDK equivalent
// Compile: vil compile --from swift --input 039-basic-observer-dashboard/main.swift --release

let server = VilServer(name: "observer-demo", port: 8080)
let demo = ServiceProcess(name: "demo")
demo.endpoint(method: "GET", path: "/hello", handler: "hello")
demo.endpoint(method: "POST", path: "/echo", handler: "echo")
server.service(demo)
server.compile()
