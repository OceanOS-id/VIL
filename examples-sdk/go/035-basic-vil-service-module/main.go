// 035-basic-vil-service-module — Go SDK equivalent
// Compile: vil compile --from go --input 035-basic-vil-service-module/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
