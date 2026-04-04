// 031-basic-mesh-routing — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 031-basic-mesh-routing/main.kt --release

fun main() {
    val server = VilServer("banking-transaction-mesh", 8080)
    val teller = ServiceProcess("teller")
    teller.endpoint("GET", "/ping", "teller_ping")
    teller.endpoint("POST", "/submit", "teller_submit")
    server.service(teller)
    val fraud_check = ServiceProcess("fraud_check")
    fraud_check.endpoint("POST", "/analyze", "fraud_process")
    server.service(fraud_check)
    val core_banking = ServiceProcess("core_banking")
    core_banking.endpoint("POST", "/post", "core_banking_post")
    server.service(core_banking)
    val notification = ServiceProcess("notification")
    notification.endpoint("GET", "/send", "notification_send")
    server.service(notification)
    server.compile()
}
