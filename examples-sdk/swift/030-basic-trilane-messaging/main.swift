// 030-basic-trilane-messaging — Swift SDK equivalent
// Compile: vil compile --from swift --input 030-basic-trilane-messaging/main.swift --release

let server = VilServer(name: "ecommerce-order-pipeline", port: 8080)
let gateway = ServiceProcess(name: "gateway")
server.service(gateway)
let fulfillment = ServiceProcess(name: "fulfillment")
fulfillment.endpoint(method: "GET", path: "/status", handler: "fulfillment_status")
server.service(fulfillment)
server.compile()
