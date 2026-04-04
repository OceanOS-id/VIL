// 401-agent-calculator — Go SDK equivalent
// Compile: vil compile --from go --input 401-agent-calculator/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("calculator-agent", 3120)

	calc_agent := vil.NewService("calc-agent")
	calc_agent.Endpoint("POST", "/calc", "calc_handler")
	s.Service(calc_agent)

	s.Compile()
}
