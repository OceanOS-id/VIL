// 013-basic-nats-worker — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 013-basic-nats-worker/main.kt --release

fun main() {
    val server = VilServer("nats-worker", 8080)
    val nats = ServiceProcess("nats")
    nats.endpoint("GET", "/nats/config", "nats_config")
    nats.endpoint("POST", "/nats/publish", "nats_publish")
    nats.endpoint("GET", "/nats/jetstream", "jetstream_info")
    nats.endpoint("GET", "/nats/kv", "kv_demo")
    server.service(nats)
    val root = ServiceProcess("root")
    server.service(root)
    server.compile()
}
