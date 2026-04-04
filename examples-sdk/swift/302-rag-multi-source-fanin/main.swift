// 302-rag-multi-source-fanin — Swift SDK equivalent
// Compile: vil compile --from swift --input 302-rag-multi-source-fanin/main.swift --release

let server = VilServer(name: "rag-multi-source-fanin", port: 3111)
server.compile()
