// 506-villog-structured-events — Go SDK equivalent
// Compile: vil compile --from go --input 506-villog-structured-events/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
