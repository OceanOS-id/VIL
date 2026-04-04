// 002-basic-vilapp-gateway — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 002-basic-vilapp-gateway/main.kt --release

fun main() {
    val server = VilServer("vil-app-gateway", 3081)
    val gw = ServiceProcess("gw")
    gw.endpoint("POST", "/trigger", "trigger_handler")
    server.service(gw)
    server.compile()
}
