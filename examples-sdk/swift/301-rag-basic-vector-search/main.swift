// 301-rag-basic-vector-search — Swift SDK equivalent
// Compile: vil compile --from swift --input 301-rag-basic-vector-search/main.swift --release

let server = VilServer(name: "rag-basic-vector-search", port: 3110)
let rag_basic = ServiceProcess(name: "rag-basic")
rag_basic.endpoint(method: "POST", path: "/rag", handler: "rag_handler")
server.service(rag_basic)
server.compile()
