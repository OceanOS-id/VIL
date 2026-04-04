// 002-basic-vilapp-gateway — Java SDK equivalent
// Compile: vil compile --from java --input 002-basic-vilapp-gateway/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("vil-app-gateway", 3081);
        ServiceProcess gw = new ServiceProcess("gw");
        gw.endpoint("POST", "/trigger", "trigger_handler");
        server.service(gw);
        server.compile();
    }
}
