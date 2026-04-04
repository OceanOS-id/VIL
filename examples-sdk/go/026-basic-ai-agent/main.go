// 026-basic-ai-agent — Go SDK equivalent
// Compile: vil compile --from go --input 026-basic-ai-agent/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("ai-agent", 8080)

	agent := vil.NewService("agent")
	agent.Endpoint("POST", "/agent", "agent_handler")
	s.Service(agent)

	s.Compile()
}
