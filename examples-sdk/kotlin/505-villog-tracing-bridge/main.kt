// 505-villog-tracing-bridge — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 505-villog-tracing-bridge/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
