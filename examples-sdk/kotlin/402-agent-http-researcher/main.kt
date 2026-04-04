// 402-agent-http-researcher — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 402-agent-http-researcher/main.kt --release

fun main() {
    val server = VilServer("http-researcher-agent", 3121)
    val research_agent = ServiceProcess("research-agent")
    research_agent.endpoint("POST", "/research", "research_handler")
    research_agent.endpoint("GET", "/products", "products_handler")
    server.service(research_agent)
    server.compile()
}
