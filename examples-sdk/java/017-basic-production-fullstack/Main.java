// 017-basic-production-fullstack — Java SDK equivalent
// Compile: vil compile --from java --input 017-basic-production-fullstack/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("production-fullstack", 8080);
        ServiceProcess fullstack = new ServiceProcess("fullstack");
        fullstack.endpoint("GET", "/stack", "stack_info");
        fullstack.endpoint("GET", "/config", "full_config");
        fullstack.endpoint("GET", "/sprints", "sprints");
        fullstack.endpoint("GET", "/middleware", "middleware_info");
        server.service(fullstack);
        ServiceProcess admin = new ServiceProcess("admin");
        admin.endpoint("GET", "/config", "full_config");
        server.service(admin);
        ServiceProcess root = new ServiceProcess("root");
        server.service(root);
        server.compile();
    }
}
