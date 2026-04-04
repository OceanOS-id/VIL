// 003-basic-hello-server — Java SDK equivalent
// Compile: vil compile --from java --input 003-basic-hello-server/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("vil-basic-hello-server", 8080);
        ServiceProcess gw = new ServiceProcess("gw");
        gw.endpoint("POST", "/transform", "transform");
        gw.endpoint("POST", "/echo", "echo");
        gw.endpoint("GET", "/health", "health");
        server.service(gw);
        server.compile();
    }
}
