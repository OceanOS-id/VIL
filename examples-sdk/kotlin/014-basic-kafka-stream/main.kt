// 014-basic-kafka-stream — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 014-basic-kafka-stream/main.kt --release

fun main() {
    val server = VilServer("kafka-stream", 8080)
    val kafka = ServiceProcess("kafka")
    kafka.endpoint("GET", "/kafka/config", "kafka_config")
    kafka.endpoint("POST", "/kafka/produce", "kafka_produce")
    kafka.endpoint("GET", "/kafka/consumer", "consumer_info")
    kafka.endpoint("GET", "/kafka/bridge", "bridge_status")
    server.service(kafka)
    val root = ServiceProcess("root")
    server.service(root)
    server.compile()
}
