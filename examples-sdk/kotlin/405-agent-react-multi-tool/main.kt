// 405-agent-react-multi-tool — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 405-agent-react-multi-tool/main.kt --release

fun main() {
    val server = VilServer("react-multi-tool-agent", 3124)
    val react_agent = ServiceProcess("react-agent")
    react_agent.endpoint("POST", "/react", "react_handler")
    server.service(react_agent)
    server.compile()
}
