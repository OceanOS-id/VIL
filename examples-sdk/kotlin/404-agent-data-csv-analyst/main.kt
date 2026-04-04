// 404-agent-data-csv-analyst — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 404-agent-data-csv-analyst/main.kt --release

fun main() {
    val server = VilServer("csv-analyst-agent", 3123)
    val csv_analyst_agent = ServiceProcess("csv-analyst-agent")
    csv_analyst_agent.endpoint("POST", "/csv-analyze", "csv_analyze_handler")
    server.service(csv_analyst_agent)
    server.compile()
}
