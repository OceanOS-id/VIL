// 403-agent-code-file-reviewer — Swift SDK equivalent
// Compile: vil compile --from swift --input 403-agent-code-file-reviewer/main.swift --release

let server = VilServer(name: "code-file-reviewer-agent", port: 3122)
let code_review_agent = ServiceProcess(name: "code-review-agent")
code_review_agent.endpoint(method: "POST", path: "/code-review", handler: "code_review_handler")
server.service(code_review_agent)
server.compile()
