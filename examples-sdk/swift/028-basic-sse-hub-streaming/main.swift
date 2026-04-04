// 028-basic-sse-hub-streaming — Swift SDK equivalent
// Compile: vil compile --from swift --input 028-basic-sse-hub-streaming/main.swift --release

let server = VilServer(name: "sse-hub-demo", port: 8080)
let events = ServiceProcess(name: "events")
events.endpoint(method: "POST", path: "/publish", handler: "publish")
events.endpoint(method: "GET", path: "/stream", handler: "stream")
events.endpoint(method: "GET", path: "/stats", handler: "stats")
server.service(events)
server.compile()
