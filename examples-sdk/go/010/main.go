// 010 — WebSocket Chat
// Equivalent to: examples/010-basic-websocket-chat (Rust)
// Compile: vil compile --from go --input 010/main.go --release
package main

import (
	"fmt"
	"os"

	vil "github.com/OceanOS-id/vil-go"
)

func main() {
	server := vil.NewServer("websocket-chat", 8080)

	// -- WebSocket events -----------------------------------------------------
	server.WsEvent("chat_message", vil.WsEventOpts{
		Topic:  "chat.message",
		Fields: map[string]string{"from": "String", "message": "String", "timestamp": "String"},
	})
	server.WsEvent("user_joined", vil.WsEventOpts{
		Topic:  "chat.presence",
		Fields: map[string]string{"username": "String"},
	})
	server.WsEvent("user_left", vil.WsEventOpts{
		Topic:  "chat.presence",
		Fields: map[string]string{"username": "String"},
	})

	// -- ServiceProcess: chat (prefix: /api/chat) -----------------------------
	chat := vil.NewServiceProcess("chat")
	chat.Endpoint("GET", "/", "index")
	chat.EndpointWs("GET", "/ws", "ws_handler")
	chat.Endpoint("GET", "/stats", "stats")
	server.Service(chat, "/api/chat")

	// -- Emit / compile -------------------------------------------------------
	if os.Getenv("VIL_COMPILE_MODE") == "manifest" {
		fmt.Println(server.ToYAML())
	} else {
		server.Compile()
	}
}
