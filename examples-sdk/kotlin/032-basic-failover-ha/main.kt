// 032-basic-failover-ha — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 032-basic-failover-ha/main.kt --release

fun main() {
    val server = VilServer("payment-gateway-ha", 8080)
    val primary = ServiceProcess("primary")
    primary.endpoint("GET", "/health", "primary_health")
    primary.endpoint("POST", "/charge", "primary_charge")
    server.service(primary)
    val backup = ServiceProcess("backup")
    backup.endpoint("GET", "/health", "backup_health")
    backup.endpoint("POST", "/charge", "backup_charge")
    server.service(backup)
    server.compile()
}
