// 004-basic-rest-crud — Swift SDK equivalent
// Compile: vil compile --from swift --input 004-basic-rest-crud/main.swift --release

let server = VilServer(name: "crud-vilorm", port: 8080)
let tasks = ServiceProcess(name: "tasks")
tasks.endpoint(method: "GET", path: "/tasks", handler: "list_tasks")
tasks.endpoint(method: "POST", path: "/tasks", handler: "create_task")
tasks.endpoint(method: "GET", path: "/tasks/stats", handler: "task_stats")
tasks.endpoint(method: "GET", path: "/tasks/:id", handler: "get_task")
tasks.endpoint(method: "PUT", path: "/tasks/:id", handler: "update_task")
tasks.endpoint(method: "DELETE", path: "/tasks/:id", handler: "delete_task")
server.service(tasks)
server.compile()
