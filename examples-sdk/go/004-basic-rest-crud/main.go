// 004-basic-rest-crud — Go SDK equivalent
// Compile: vil compile --from go --input 004-basic-rest-crud/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("crud-vilorm", 8080)

	tasks := vil.NewService("tasks")
	tasks.Endpoint("GET", "/tasks", "list_tasks")
	tasks.Endpoint("POST", "/tasks", "create_task")
	tasks.Endpoint("GET", "/tasks/stats", "task_stats")
	tasks.Endpoint("GET", "/tasks/:id", "get_task")
	tasks.Endpoint("PUT", "/tasks/:id", "update_task")
	tasks.Endpoint("DELETE", "/tasks/:id", "delete_task")
	s.Service(tasks)

	s.Compile()
}
