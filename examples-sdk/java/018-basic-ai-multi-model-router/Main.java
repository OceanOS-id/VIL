// 018-basic-ai-multi-model-router — Java SDK equivalent
// Compile: vil compile --from java --input 018-basic-ai-multi-model-router/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("ai-multi-model-router", 3085);
        ServiceProcess router = new ServiceProcess("router");
        router.endpoint("POST", "/route", "route_handler");
        server.service(router);
        server.compile();
    }
}
