// 503-villog-multi-drain — Go SDK equivalent
// Compile: vil compile --from go --input 503-villog-multi-drain/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
