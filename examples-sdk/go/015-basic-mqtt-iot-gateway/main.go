// 015-basic-mqtt-iot-gateway — Go SDK equivalent
// Compile: vil compile --from go --input 015-basic-mqtt-iot-gateway/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("mqtt-iot-gateway", 8080)

	mqtt_iot := vil.NewService("mqtt-iot")
	mqtt_iot.Endpoint("POST", "/sensors/data", "receive_sensor_data")
	mqtt_iot.Endpoint("GET", "/sensors", "list_sensors")
	mqtt_iot.Endpoint("GET", "/mqtt/config", "mqtt_config")
	mqtt_iot.Endpoint("GET", "/mqtt/topics", "mqtt_topics")
	s.Service(mqtt_iot)

	root := vil.NewService("root")
	s.Service(root)

	s.Compile()
}
