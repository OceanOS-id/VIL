// 027-basic-vilserver-minimal — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 027-basic-vilserver-minimal/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
