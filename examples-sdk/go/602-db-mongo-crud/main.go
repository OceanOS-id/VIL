// 602-db-mongo-crud — Go SDK equivalent
// Compile: vil compile --from go --input 602-db-mongo-crud/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
