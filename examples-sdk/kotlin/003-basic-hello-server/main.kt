// 003-basic-hello-server — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 003-basic-hello-server/main.kt --release

fun main() {
    val server = VilServer("vil-basic-hello-server", 8080)
    val gw = ServiceProcess("gw")
    gw.endpoint("POST", "/transform", "transform")
    gw.endpoint("POST", "/echo", "echo")
    gw.endpoint("GET", "/health", "health")
    server.service(gw)
    server.compile()
}
