// 303-rag-hybrid-exact-semantic — Swift SDK equivalent
// Compile: vil compile --from swift --input 303-rag-hybrid-exact-semantic/main.swift --release

let server = VilServer(name: "rag-hybrid-exact-semantic", port: 3112)
let rag_hybrid = ServiceProcess(name: "rag-hybrid")
rag_hybrid.endpoint(method: "POST", path: "/hybrid", handler: "hybrid_handler")
server.service(rag_hybrid)
server.compile()
