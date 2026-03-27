// 004 — REST CRUD (ServiceProcess + State)
// Equivalent to: examples/004-basic-rest-crud (Rust)
// Compile: vil compile --from go --input 004/main.go --release
package main

import (
	"fmt"
	"os"

	vil "github.com/OceanOS-id/vil-go"
)

func main() {
	server := vil.NewServer("crud-service", 8080)

	// -- Semantic types -------------------------------------------------------
	server.SemanticType("TaskState", "state", map[string]string{
		"task_count":    "u32",
		"last_modified": "u64",
	})
	server.Fault("CrudFault", []string{"NotFound", "InvalidInput", "Conflict"})

	// -- ServiceProcess: tasks (prefix: /api) ---------------------------------
	tasks := vil.NewServiceProcess("tasks")
	tasks.Endpoint("GET", "/tasks", "list_tasks")
	tasks.Endpoint("POST", "/tasks", "create_task")
	tasks.Endpoint("GET", "/tasks/:id", "get_task")
	tasks.Endpoint("PUT", "/tasks/:id", "update_task")
	tasks.Endpoint("DELETE", "/tasks/:id", "delete_task")
	server.Service(tasks, "/api")

	// -- Emit / compile -------------------------------------------------------
	if os.Getenv("VIL_COMPILE_MODE") == "manifest" {
		fmt.Println(server.ToYAML())
	} else {
		server.Compile()
	}
}
