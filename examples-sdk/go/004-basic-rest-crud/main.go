// 004 — REST CRUD (ServiceProcess + State)
// Equivalent to: examples/004-basic-rest-crud (Rust)
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	server := vil.NewServer("crud-vilorm", 8080)

	tasks := vil.NewService("tasks")
	tasks.Endpoint("GET", "/tasks", "list_tasks")
	tasks.Endpoint("POST", "/tasks", "create_task")
	tasks.Endpoint("GET", "/tasks/stats", "task_stats")
	tasks.Endpoint("GET", "/tasks/:id", "get_task")
	tasks.Endpoint("PUT", "/tasks/:id", "update_task")
	tasks.Endpoint("DELETE", "/tasks/:id", "delete_task")
	server.Service(tasks)

	server.Compile()
}
