// 004-basic-rest-crud — Java SDK equivalent
// Compile: vil compile --from java --input 004-basic-rest-crud/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("crud-vilorm", 8080);
        ServiceProcess tasks = new ServiceProcess("tasks");
        tasks.endpoint("GET", "/tasks", "list_tasks");
        tasks.endpoint("POST", "/tasks", "create_task");
        tasks.endpoint("GET", "/tasks/stats", "task_stats");
        tasks.endpoint("GET", "/tasks/:id", "get_task");
        tasks.endpoint("PUT", "/tasks/:id", "update_task");
        tasks.endpoint("DELETE", "/tasks/:id", "delete_task");
        server.service(tasks);
        server.compile();
    }
}
