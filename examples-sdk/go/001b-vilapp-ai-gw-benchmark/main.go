// 001b-vilapp-ai-gw-benchmark — Go SDK equivalent
// Compile: vil compile --from go --input 001b-vilapp-ai-gw-benchmark/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("ai-gw-bench", 3081)

	gw := vil.NewService("gw")
	s.Service(gw)

	s.Compile()
}
