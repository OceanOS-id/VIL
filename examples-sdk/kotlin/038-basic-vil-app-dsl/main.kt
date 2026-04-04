// 038-basic-vil-app-dsl — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 038-basic-vil-app-dsl/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
