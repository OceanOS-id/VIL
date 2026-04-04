// 703-protocol-soap-client — Go SDK equivalent
// Compile: vil compile --from go --input 703-protocol-soap-client/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
