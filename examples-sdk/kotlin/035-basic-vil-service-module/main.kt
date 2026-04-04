// 035-basic-vil-service-module — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 035-basic-vil-service-module/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
