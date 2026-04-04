// 507-villog-bench-file-drain — Go SDK equivalent
// Compile: vil compile --from go --input 507-villog-bench-file-drain/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
