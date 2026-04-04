// 403-agent-code-file-reviewer — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 403-agent-code-file-reviewer/main.kt --release

fun main() {
    val server = VilServer("code-file-reviewer-agent", 3122)
    val code_review_agent = ServiceProcess("code-review-agent")
    code_review_agent.endpoint("POST", "/code-review", "code_review_handler")
    server.service(code_review_agent)
    server.compile()
}
