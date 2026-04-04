// 402-agent-http-researcher — Go SDK equivalent
// Compile: vil compile --from go --input 402-agent-http-researcher/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("http-researcher-agent", 3121)

	research_agent := vil.NewService("research-agent")
	research_agent.Endpoint("POST", "/research", "research_handler")
	research_agent.Endpoint("GET", "/products", "products_handler")
	s.Service(research_agent)

	s.Compile()
}
