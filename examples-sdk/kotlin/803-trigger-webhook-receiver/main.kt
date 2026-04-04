// 803-trigger-webhook-receiver — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 803-trigger-webhook-receiver/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
