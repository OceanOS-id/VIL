// 004-basic-rest-crud — C# SDK equivalent
// Compile: vil compile --from csharp --input 004-basic-rest-crud/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("crud-vilorm", 8080);
var tasks = new ServiceProcess("tasks");
tasks.Endpoint("GET", "/tasks", "list_tasks");
tasks.Endpoint("POST", "/tasks", "create_task");
tasks.Endpoint("GET", "/tasks/stats", "task_stats");
tasks.Endpoint("GET", "/tasks/:id", "get_task");
tasks.Endpoint("PUT", "/tasks/:id", "update_task");
tasks.Endpoint("DELETE", "/tasks/:id", "delete_task");
server.Service(tasks);
server.Compile();
