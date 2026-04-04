// 001b-vilapp-ai-gw-benchmark — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 001b-vilapp-ai-gw-benchmark/main.kt --release

fun main() {
    val server = VilServer("ai-gw-bench", 3081)
    val gw = ServiceProcess("gw")
    server.service(gw)
    server.compile()
}
