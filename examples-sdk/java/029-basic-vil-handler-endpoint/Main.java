// 029-basic-vil-handler-endpoint — Java SDK equivalent
// Compile: vil compile --from java --input 029-basic-vil-handler-endpoint/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("macro-demo", 8080);
        ServiceProcess demo = new ServiceProcess("demo");
        demo.endpoint("GET", "/plain", "plain_handler");
        demo.endpoint("GET", "/handled", "handled_handler");
        demo.endpoint("POST", "/endpoint", "endpoint_handler");
        server.service(demo);
        server.compile();
    }
}
