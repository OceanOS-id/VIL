// 029-basic-vil-handler-endpoint — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 029-basic-vil-handler-endpoint/main.kt --release

fun main() {
    val server = VilServer("macro-demo", 8080)
    val demo = ServiceProcess("demo")
    demo.endpoint("GET", "/plain", "plain_handler")
    demo.endpoint("GET", "/handled", "handled_handler")
    demo.endpoint("POST", "/endpoint", "endpoint_handler")
    server.service(demo)
    server.compile()
}
