// 206-llm-decision-routing — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 206-llm-decision-routing/main.kt --release

fun main() {
    val server = VilServer("insurance-underwriting-ai", 3116)
    val underwriter = ServiceProcess("underwriter")
    server.service(underwriter)
    server.compile()
}
