// 702-mq-sqs-send-receive — Go SDK equivalent
// Compile: vil compile --from go --input 702-mq-sqs-send-receive/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("app", 8080)
	s.Compile()
}
