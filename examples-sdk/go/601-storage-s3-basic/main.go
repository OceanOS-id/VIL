// 601-storage-s3-basic — Go SDK equivalent
// Compile: vil compile --from go --input 601-storage-s3-basic/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
