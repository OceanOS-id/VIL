// 802-trigger-fs-watcher — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 802-trigger-fs-watcher/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
