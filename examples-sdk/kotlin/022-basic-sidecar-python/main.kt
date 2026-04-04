// 022-basic-sidecar-python — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 022-basic-sidecar-python/main.kt --release

fun main() {
    val server = VilServer("sidecar-python-example", 8080)
    val fraud = ServiceProcess("fraud")
    fraud.endpoint("GET", "/status", "fraud_status")
    fraud.endpoint("POST", "/check", "fraud_check")
    server.service(fraud)
    val root = ServiceProcess("root")
    root.endpoint("GET", "/", "index")
    server.service(root)
    server.compile()
}
