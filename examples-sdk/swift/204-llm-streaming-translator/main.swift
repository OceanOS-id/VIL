// 204-llm-streaming-translator — Swift SDK equivalent
// Compile: vil compile --from swift --input 204-llm-streaming-translator/main.swift --release

let server = VilServer(name: "llm-streaming-translator", port: 3103)
let translator = ServiceProcess(name: "translator")
server.service(translator)
server.compile()
