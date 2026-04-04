// 406-agent-vil-handler-shm — Java SDK equivalent
// Compile: vil compile --from java --input 406-agent-vil-handler-shm/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("fraud-detection-agent", 3126);
        ServiceProcess fraud_agent = new ServiceProcess("fraud-agent");
        fraud_agent.endpoint("POST", "/detect", "detect_fraud");
        fraud_agent.endpoint("GET", "/health", "health");
        server.service(fraud_agent);
        server.compile();
    }
}
