// 507-villog-bench-file-drain — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 507-villog-bench-file-drain/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
