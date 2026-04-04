// 024-basic-llm-chat — Go SDK equivalent
// Compile: vil compile --from go --input 024-basic-llm-chat/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("llm-chat", 8080)

	chat := vil.NewService("chat")
	chat.Endpoint("POST", "/chat", "chat_handler")
	s.Service(chat)

	s.Compile()
}
