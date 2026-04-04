// 017-basic-production-fullstack — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 017-basic-production-fullstack/main.kt --release

fun main() {
    val server = VilServer("production-fullstack", 8080)
    val fullstack = ServiceProcess("fullstack")
    fullstack.endpoint("GET", "/stack", "stack_info")
    fullstack.endpoint("GET", "/config", "full_config")
    fullstack.endpoint("GET", "/sprints", "sprints")
    fullstack.endpoint("GET", "/middleware", "middleware_info")
    server.service(fullstack)
    val admin = ServiceProcess("admin")
    admin.endpoint("GET", "/config", "full_config")
    server.service(admin)
    val root = ServiceProcess("root")
    server.service(root)
    server.compile()
}
