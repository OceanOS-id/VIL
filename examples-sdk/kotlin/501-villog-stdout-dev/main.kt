// 501-villog-stdout-dev — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 501-villog-stdout-dev/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
