// 034-basic-blocking-task — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 034-basic-blocking-task/main.kt --release

fun main() {
    val server = VilServer("credit-risk-scoring-engine", 8080)
    val risk_engine = ServiceProcess("risk-engine")
    risk_engine.endpoint("POST", "/risk/assess", "assess_risk")
    risk_engine.endpoint("GET", "/risk/health", "risk_health")
    server.service(risk_engine)
    server.compile()
}
