// 203-llm-code-review-with-tools — Swift SDK equivalent
// Compile: vil compile --from swift --input 203-llm-code-review-with-tools/main.swift --release

let server = VilServer(name: "llm-code-review-tools", port: 3102)
let code_review = ServiceProcess(name: "code-review")
code_review.endpoint(method: "POST", path: "/code/review", handler: "code_review_handler")
server.service(code_review)
server.compile()
