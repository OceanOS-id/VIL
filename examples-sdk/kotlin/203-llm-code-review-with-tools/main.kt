// 203-llm-code-review-with-tools — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 203-llm-code-review-with-tools/main.kt --release

fun main() {
    val server = VilServer("llm-code-review-tools", 3102)
    val code_review = ServiceProcess("code-review")
    code_review.endpoint("POST", "/code/review", "code_review_handler")
    server.service(code_review)
    server.compile()
}
