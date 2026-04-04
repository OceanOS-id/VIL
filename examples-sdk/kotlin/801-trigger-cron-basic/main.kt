// 801-trigger-cron-basic — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 801-trigger-cron-basic/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
