// 304-rag-citation-extraction — Swift SDK equivalent
// Compile: vil compile --from swift --input 304-rag-citation-extraction/main.swift --release

let server = VilServer(name: "rag-citation-extraction", port: 3113)
let rag_citation = ServiceProcess(name: "rag-citation")
rag_citation.endpoint(method: "POST", path: "/cited-rag", handler: "cited_rag_handler")
server.service(rag_citation)
server.compile()
