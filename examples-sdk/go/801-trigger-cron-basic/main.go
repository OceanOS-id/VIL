// 801-trigger-cron-basic — Go SDK equivalent
// Compile: vil compile --from go --input 801-trigger-cron-basic/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
