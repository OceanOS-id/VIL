// 306-rag-ai-event-tracking — Swift SDK equivalent
// Compile: vil compile --from swift --input 306-rag-ai-event-tracking/main.swift --release

let server = VilServer(name: "customer-support-rag", port: 3116)
let support = ServiceProcess(name: "support")
support.endpoint(method: "POST", path: "/support/ask", handler: "answer_question")
server.service(support)
server.compile()
