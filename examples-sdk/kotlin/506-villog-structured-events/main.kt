// 506-villog-structured-events — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 506-villog-structured-events/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
