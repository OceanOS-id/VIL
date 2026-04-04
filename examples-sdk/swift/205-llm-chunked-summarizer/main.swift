// 205-llm-chunked-summarizer — Swift SDK equivalent
// Compile: vil compile --from swift --input 205-llm-chunked-summarizer/main.swift --release

let server = VilServer(name: "ChunkedSummarizerPipeline", port: 8080)
server.compile()
