// 030-basic-trilane-messaging — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 030-basic-trilane-messaging/main.kt --release

fun main() {
    val server = VilServer("ecommerce-order-pipeline", 8080)
    val gateway = ServiceProcess("gateway")
    server.service(gateway)
    val fulfillment = ServiceProcess("fulfillment")
    fulfillment.endpoint("GET", "/status", "fulfillment_status")
    server.service(fulfillment)
    server.compile()
}
