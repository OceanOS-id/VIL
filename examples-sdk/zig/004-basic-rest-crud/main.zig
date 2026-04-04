// 004-basic-rest-crud — Zig SDK equivalent
// Compile: vil compile --from zig --input 004-basic-rest-crud/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("crud-vilorm", 8080);
    var tasks = vil.Service.init("tasks");
    tasks.endpoint("GET", "/tasks", "list_tasks");
    tasks.endpoint("POST", "/tasks", "create_task");
    tasks.endpoint("GET", "/tasks/stats", "task_stats");
    tasks.endpoint("GET", "/tasks/:id", "get_task");
    tasks.endpoint("PUT", "/tasks/:id", "update_task");
    tasks.endpoint("DELETE", "/tasks/:id", "delete_task");
    server.service(&tasks);
    server.compile();
}
