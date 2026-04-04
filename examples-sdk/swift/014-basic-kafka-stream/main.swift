// 014-basic-kafka-stream — Swift SDK equivalent
// Compile: vil compile --from swift --input 014-basic-kafka-stream/main.swift --release

let server = VilServer(name: "kafka-stream", port: 8080)
let kafka = ServiceProcess(name: "kafka")
kafka.endpoint(method: "GET", path: "/kafka/config", handler: "kafka_config")
kafka.endpoint(method: "POST", path: "/kafka/produce", handler: "kafka_produce")
kafka.endpoint(method: "GET", path: "/kafka/consumer", handler: "consumer_info")
kafka.endpoint(method: "GET", path: "/kafka/bridge", handler: "bridge_status")
server.service(kafka)
let root = ServiceProcess(name: "root")
server.service(root)
server.compile()
