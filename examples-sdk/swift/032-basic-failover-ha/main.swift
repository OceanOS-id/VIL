// 032-basic-failover-ha — Swift SDK equivalent
// Compile: vil compile --from swift --input 032-basic-failover-ha/main.swift --release

let server = VilServer(name: "payment-gateway-ha", port: 8080)
let primary = ServiceProcess(name: "primary")
primary.endpoint(method: "GET", path: "/health", handler: "primary_health")
primary.endpoint(method: "POST", path: "/charge", handler: "primary_charge")
server.service(primary)
let backup = ServiceProcess(name: "backup")
backup.endpoint(method: "GET", path: "/health", handler: "backup_health")
backup.endpoint(method: "POST", path: "/charge", handler: "backup_charge")
server.service(backup)
server.compile()
