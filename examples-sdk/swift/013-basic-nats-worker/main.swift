// 013-basic-nats-worker — Swift SDK equivalent
// Compile: vil compile --from swift --input 013-basic-nats-worker/main.swift --release

let server = VilServer(name: "nats-worker", port: 8080)
let nats = ServiceProcess(name: "nats")
nats.endpoint(method: "GET", path: "/nats/config", handler: "nats_config")
nats.endpoint(method: "POST", path: "/nats/publish", handler: "nats_publish")
nats.endpoint(method: "GET", path: "/nats/jetstream", handler: "jetstream_info")
nats.endpoint(method: "GET", path: "/nats/kv", handler: "kv_demo")
server.service(nats)
let root = ServiceProcess(name: "root")
server.service(root)
server.compile()
