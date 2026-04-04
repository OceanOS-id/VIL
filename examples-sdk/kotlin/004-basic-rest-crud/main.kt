// 004-basic-rest-crud — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 004-basic-rest-crud/main.kt --release

fun main() {
    val server = VilServer("crud-vilorm", 8080)
    val tasks = ServiceProcess("tasks")
    tasks.endpoint("GET", "/tasks", "list_tasks")
    tasks.endpoint("POST", "/tasks", "create_task")
    tasks.endpoint("GET", "/tasks/stats", "task_stats")
    tasks.endpoint("GET", "/tasks/:id", "get_task")
    tasks.endpoint("PUT", "/tasks/:id", "update_task")
    tasks.endpoint("DELETE", "/tasks/:id", "delete_task")
    server.service(tasks)
    server.compile()
}
