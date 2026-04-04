// 503-villog-multi-drain — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 503-villog-multi-drain/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
