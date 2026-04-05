// 004-basic-rest-crud — Go SDK equivalent
// Compile: vil compile --from go --input 004-basic-rest-crud/main.go --release
//
// Task CRUD with SQLite. Full REST API: list, create, get, update, delete, stats.
// Switch mode: VIL_MODE=sidecar (default) / VIL_MODE=wasm

package main

import (
	"example/004-basic-rest-crud/handlers"

	vil "github.com/OceanOS-id/vil-go"
)

var mode = vil.ModeFromEnv()

var ListTasks = vil.Handler("HandleListTasks", mode, "shm", handlers.HandleListTasks)
var CreateTask = vil.Handler("HandleCreateTask", mode, "shm", handlers.HandleCreateTask)
var GetTask = vil.Handler("HandleGetTask", mode, "shm", handlers.HandleGetTask)
var UpdateTask = vil.Handler("HandleUpdateTask", mode, "shm", handlers.HandleUpdateTask)
var DeleteTask = vil.Handler("HandleDeleteTask", mode, "shm", handlers.HandleDeleteTask)
var TaskStats = vil.Handler("HandleTaskStats", mode, "shm", handlers.HandleTaskStats)

func main() {
	vil.Run(ListTasks, CreateTask, GetTask, UpdateTask, DeleteTask, TaskStats)

	s := vil.NewServer("crud-vilorm", 8080)
	tasks := vil.NewService("tasks")

	tasks.Endpoint("GET", "/tasks", ListTasks)
	tasks.Endpoint("POST", "/tasks", CreateTask)
	tasks.Endpoint("GET", "/tasks/stats", TaskStats)
	tasks.Endpoint("GET", "/tasks/:id", GetTask)
	tasks.Endpoint("PUT", "/tasks/:id", UpdateTask)
	tasks.Endpoint("DELETE", "/tasks/:id", DeleteTask)

	s.Service(tasks)
	s.Compile()
}
