// 024-basic-llm-chat — Swift SDK equivalent
// Compile: vil compile --from swift --input 024-basic-llm-chat/main.swift --release

let server = VilServer(name: "llm-chat", port: 8080)
let chat = ServiceProcess(name: "chat")
chat.endpoint(method: "POST", path: "/chat", handler: "chat_handler")
server.service(chat)
server.compile()
