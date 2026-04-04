// 401-agent-calculator — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 401-agent-calculator/main.kt --release

fun main() {
    val server = VilServer("calculator-agent", 3120)
    val calc_agent = ServiceProcess("calc-agent")
    calc_agent.endpoint("POST", "/calc", "calc_handler")
    server.service(calc_agent)
    server.compile()
}
