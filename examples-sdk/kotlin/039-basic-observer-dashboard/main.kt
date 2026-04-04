// 039-basic-observer-dashboard — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 039-basic-observer-dashboard/main.kt --release

fun main() {
    val server = VilServer("observer-demo", 8080)
    val demo = ServiceProcess("demo")
    demo.endpoint("GET", "/hello", "hello")
    demo.endpoint("POST", "/echo", "echo")
    server.service(demo)
    server.compile()
}
