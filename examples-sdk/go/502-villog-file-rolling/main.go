// 502-villog-file-rolling — Go SDK equivalent
// Compile: vil compile --from go --input 502-villog-file-rolling/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
