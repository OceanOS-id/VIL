// 026-basic-ai-agent — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 026-basic-ai-agent/main.kt --release

fun main() {
    val server = VilServer("ai-agent", 8080)
    val agent = ServiceProcess("agent")
    agent.endpoint("POST", "/agent", "agent_handler")
    server.service(agent)
    server.compile()
}
