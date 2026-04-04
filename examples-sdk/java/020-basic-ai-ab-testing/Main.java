// 020-basic-ai-ab-testing — Java SDK equivalent
// Compile: vil compile --from java --input 020-basic-ai-ab-testing/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("ai-ab-testing-gateway", 8080);
        ServiceProcess ab = new ServiceProcess("ab");
        ab.endpoint("POST", "/infer", "infer");
        ab.endpoint("GET", "/metrics", "metrics");
        ab.endpoint("POST", "/config", "update_config");
        server.service(ab);
        ServiceProcess root = new ServiceProcess("root");
        root.endpoint("GET", "/", "index");
        server.service(root);
        server.compile();
    }
}
