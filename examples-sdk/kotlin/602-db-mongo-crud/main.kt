// 602-db-mongo-crud — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 602-db-mongo-crud/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
