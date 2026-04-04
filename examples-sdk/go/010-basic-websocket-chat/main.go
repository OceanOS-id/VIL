// 010 — WebSocket Chat
// Equivalent to: examples/010-basic-websocket-chat (Rust)
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	server := vil.NewServer("websocket-chat", 8080)

	chat := vil.NewService("chat")
	chat.Endpoint("GET", "/", "index")
	chat.Endpoint("GET", "/ws", "ws_handler")
	chat.Endpoint("GET", "/stats", "stats")
	server.Service(chat)

	server.Compile()
}
