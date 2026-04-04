// 201-llm-basic-chat — Swift SDK equivalent
// Compile: vil compile --from swift --input 201-llm-basic-chat/main.swift --release

let server = VilServer(name: "llm-basic-chat", port: 3100)
let chat = ServiceProcess(name: "chat")
chat.endpoint(method: "POST", path: "/chat", handler: "chat_handler")
server.service(chat)
server.compile()
