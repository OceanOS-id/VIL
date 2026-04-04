// 509-villog-phase1-integration — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 509-villog-phase1-integration/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
