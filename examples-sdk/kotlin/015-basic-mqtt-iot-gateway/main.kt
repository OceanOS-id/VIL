// 015-basic-mqtt-iot-gateway — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 015-basic-mqtt-iot-gateway/main.kt --release

fun main() {
    val server = VilServer("mqtt-iot-gateway", 8080)
    val mqtt_iot = ServiceProcess("mqtt-iot")
    mqtt_iot.endpoint("POST", "/sensors/data", "receive_sensor_data")
    mqtt_iot.endpoint("GET", "/sensors", "list_sensors")
    mqtt_iot.endpoint("GET", "/mqtt/config", "mqtt_config")
    mqtt_iot.endpoint("GET", "/mqtt/topics", "mqtt_topics")
    server.service(mqtt_iot)
    val root = ServiceProcess("root")
    server.service(root)
    server.compile()
}
