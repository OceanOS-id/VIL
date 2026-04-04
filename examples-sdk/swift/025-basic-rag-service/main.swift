// 025-basic-rag-service — Swift SDK equivalent
// Compile: vil compile --from swift --input 025-basic-rag-service/main.swift --release

let server = VilServer(name: "rag-service", port: 3091)
let rag = ServiceProcess(name: "rag")
rag.endpoint(method: "POST", path: "/rag", handler: "rag_handler")
server.service(rag)
server.compile()
