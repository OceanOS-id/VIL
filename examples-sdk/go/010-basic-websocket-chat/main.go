// 010-basic-websocket-chat — Go SDK equivalent
// Compile: vil compile --from go --input 010-basic-websocket-chat/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("websocket-chat", 8080)

	chat := vil.NewService("chat")
	chat.Endpoint("GET", "/", "index")
	chat.Endpoint("GET", "/ws", "ws_handler")
	chat.Endpoint("GET", "/stats", "stats")
	s.Service(chat)

	s.Compile()
}
