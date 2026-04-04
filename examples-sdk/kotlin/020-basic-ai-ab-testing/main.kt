// 020-basic-ai-ab-testing — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 020-basic-ai-ab-testing/main.kt --release

fun main() {
    val server = VilServer("ai-ab-testing-gateway", 8080)
    val ab = ServiceProcess("ab")
    ab.endpoint("POST", "/infer", "infer")
    ab.endpoint("GET", "/metrics", "metrics")
    ab.endpoint("POST", "/config", "update_config")
    server.service(ab)
    val root = ServiceProcess("root")
    root.endpoint("GET", "/", "index")
    server.service(root)
    server.compile()
}
