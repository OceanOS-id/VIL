// 508-villog-bench-multithread — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 508-villog-bench-multithread/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
