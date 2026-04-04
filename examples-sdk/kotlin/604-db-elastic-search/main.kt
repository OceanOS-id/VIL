// 604-db-elastic-search — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 604-db-elastic-search/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
