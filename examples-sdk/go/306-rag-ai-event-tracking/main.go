// 306-rag-ai-event-tracking — Go SDK equivalent
// Compile: vil compile --from go --input 306-rag-ai-event-tracking/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("customer-support-rag", 3116)

	support := vil.NewService("support")
	support.Endpoint("POST", "/support/ask", "answer_question")
	s.Service(support)

	s.Compile()
}
