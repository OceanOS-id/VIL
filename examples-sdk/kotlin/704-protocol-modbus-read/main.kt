// 704-protocol-modbus-read — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 704-protocol-modbus-read/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
