// 014-basic-kafka-stream — Go SDK equivalent
// Compile: vil compile --from go --input 014-basic-kafka-stream/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("kafka-stream", 8080)

	kafka := vil.NewService("kafka")
	kafka.Endpoint("GET", "/kafka/config", "kafka_config")
	kafka.Endpoint("POST", "/kafka/produce", "kafka_produce")
	kafka.Endpoint("GET", "/kafka/consumer", "consumer_info")
	kafka.Endpoint("GET", "/kafka/bridge", "bridge_status")
	s.Service(kafka)

	root := vil.NewService("root")
	s.Service(root)

	s.Compile()
}
