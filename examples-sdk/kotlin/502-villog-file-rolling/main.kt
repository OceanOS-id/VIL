// 502-villog-file-rolling — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 502-villog-file-rolling/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
