// 015-basic-mqtt-iot-gateway — Swift SDK equivalent
// Compile: vil compile --from swift --input 015-basic-mqtt-iot-gateway/main.swift --release

let server = VilServer(name: "mqtt-iot-gateway", port: 8080)
let mqtt_iot = ServiceProcess(name: "mqtt-iot")
mqtt_iot.endpoint(method: "POST", path: "/sensors/data", handler: "receive_sensor_data")
mqtt_iot.endpoint(method: "GET", path: "/sensors", handler: "list_sensors")
mqtt_iot.endpoint(method: "GET", path: "/mqtt/config", handler: "mqtt_config")
mqtt_iot.endpoint(method: "GET", path: "/mqtt/topics", handler: "mqtt_topics")
server.service(mqtt_iot)
let root = ServiceProcess(name: "root")
server.service(root)
server.compile()
