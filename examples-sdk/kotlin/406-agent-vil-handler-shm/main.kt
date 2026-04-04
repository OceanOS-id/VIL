// 406-agent-vil-handler-shm — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 406-agent-vil-handler-shm/main.kt --release

fun main() {
    val server = VilServer("fraud-detection-agent", 3126)
    val fraud_agent = ServiceProcess("fraud-agent")
    fraud_agent.endpoint("POST", "/detect", "detect_fraud")
    fraud_agent.endpoint("GET", "/health", "health")
    server.service(fraud_agent)
    server.compile()
}
