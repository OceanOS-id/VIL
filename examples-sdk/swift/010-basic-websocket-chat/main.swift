// 010-basic-websocket-chat — Swift SDK equivalent
// Compile: vil compile --from swift --input 010-basic-websocket-chat/main.swift --release

let server = VilServer(name: "websocket-chat", port: 8080)
let chat = ServiceProcess(name: "chat")
chat.endpoint(method: "GET", path: "/", handler: "index")
chat.endpoint(method: "GET", path: "/ws", handler: "ws_handler")
chat.endpoint(method: "GET", path: "/stats", handler: "stats")
server.service(chat)
server.compile()
