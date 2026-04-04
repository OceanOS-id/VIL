// 028-basic-sse-hub-streaming — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 028-basic-sse-hub-streaming/main.kt --release

fun main() {
    val server = VilServer("sse-hub-demo", 8080)
    val events = ServiceProcess("events")
    events.endpoint("POST", "/publish", "publish")
    events.endpoint("GET", "/stream", "stream")
    events.endpoint("GET", "/stats", "stats")
    server.service(events)
    server.compile()
}
