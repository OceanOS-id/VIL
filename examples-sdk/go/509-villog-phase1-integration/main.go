// 509-villog-phase1-integration — Go SDK equivalent
// Compile: vil compile --from go --input 509-villog-phase1-integration/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
