// 038-basic-vil-app-dsl — Go SDK equivalent
// Compile: vil compile --from go --input 038-basic-vil-app-dsl/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
