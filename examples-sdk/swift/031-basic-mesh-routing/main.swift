// 031-basic-mesh-routing — Swift SDK equivalent
// Compile: vil compile --from swift --input 031-basic-mesh-routing/main.swift --release

let server = VilServer(name: "banking-transaction-mesh", port: 8080)
let teller = ServiceProcess(name: "teller")
teller.endpoint(method: "GET", path: "/ping", handler: "teller_ping")
teller.endpoint(method: "POST", path: "/submit", handler: "teller_submit")
server.service(teller)
let fraud_check = ServiceProcess(name: "fraud_check")
fraud_check.endpoint(method: "POST", path: "/analyze", handler: "fraud_process")
server.service(fraud_check)
let core_banking = ServiceProcess(name: "core_banking")
core_banking.endpoint(method: "POST", path: "/post", handler: "core_banking_post")
server.service(core_banking)
let notification = ServiceProcess(name: "notification")
notification.endpoint(method: "GET", path: "/send", handler: "notification_send")
server.service(notification)
server.compile()
