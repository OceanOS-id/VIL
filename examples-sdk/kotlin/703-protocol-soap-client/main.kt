// 703-protocol-soap-client — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 703-protocol-soap-client/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
