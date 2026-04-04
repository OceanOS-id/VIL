// 018-basic-ai-multi-model-router — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 018-basic-ai-multi-model-router/main.kt --release

fun main() {
    val server = VilServer("ai-multi-model-router", 3085)
    val router = ServiceProcess("router")
    router.endpoint("POST", "/route", "route_handler")
    server.service(router)
    server.compile()
}
