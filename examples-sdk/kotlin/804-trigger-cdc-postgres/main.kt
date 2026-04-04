// 804-trigger-cdc-postgres — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 804-trigger-cdc-postgres/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
