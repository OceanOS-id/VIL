// 022-basic-sidecar-python — Java SDK equivalent
// Compile: vil compile --from java --input 022-basic-sidecar-python/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("sidecar-python-example", 8080);
        ServiceProcess fraud = new ServiceProcess("fraud");
        fraud.endpoint("GET", "/status", "fraud_status");
        fraud.endpoint("POST", "/check", "fraud_check");
        server.service(fraud);
        ServiceProcess root = new ServiceProcess("root");
        root.endpoint("GET", "/", "index");
        server.service(root);
        server.compile();
    }
}
