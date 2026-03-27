// 004 — REST CRUD (ServiceProcess + State)
// Equivalent to: examples/004-basic-rest-crud (Rust)
// Compile: vil compile --from java --input 004/Main.java --release
package dev.vil.examples;

import dev.vil.VilServer;
import dev.vil.ServiceProcess;
import java.util.Map;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("crud-service", 8080);

        // -- Semantic types ---------------------------------------------------
        server.semanticType("TaskState", "state", Map.of(
            "task_count", "u32",
            "last_modified", "u64"
        ));
        server.fault("CrudFault", List.of("NotFound", "InvalidInput", "Conflict"));

        // -- ServiceProcess: tasks (prefix: /api) -----------------------------
        ServiceProcess tasks = new ServiceProcess("tasks");
        tasks.endpoint("GET", "/tasks", "list_tasks");
        tasks.endpoint("POST", "/tasks", "create_task");
        tasks.endpoint("GET", "/tasks/:id", "get_task");
        tasks.endpoint("PUT", "/tasks/:id", "update_task");
        tasks.endpoint("DELETE", "/tasks/:id", "delete_task");
        server.service(tasks, "/api");

        // -- Emit / compile ---------------------------------------------------
        if ("manifest".equals(System.getenv("VIL_COMPILE_MODE"))) {
            System.out.println(server.toYaml());
        } else {
            server.compile();
        }
    }
}
