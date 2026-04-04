// 504-villog-benchmark-comparison — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 504-villog-benchmark-comparison/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
