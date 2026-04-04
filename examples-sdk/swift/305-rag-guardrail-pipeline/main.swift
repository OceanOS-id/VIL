// 305-rag-guardrail-pipeline — Swift SDK equivalent
// Compile: vil compile --from swift --input 305-rag-guardrail-pipeline/main.swift --release

let server = VilServer(name: "rag-guardrail-pipeline", port: 3114)
let rag_guardrail = ServiceProcess(name: "rag-guardrail")
rag_guardrail.endpoint(method: "POST", path: "/safe-rag", handler: "safe_rag_handler")
server.service(rag_guardrail)
server.compile()
