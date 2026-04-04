// 034-basic-blocking-task — Java SDK equivalent
// Compile: vil compile --from java --input 034-basic-blocking-task/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("credit-risk-scoring-engine", 8080);
        ServiceProcess risk_engine = new ServiceProcess("risk-engine");
        risk_engine.endpoint("POST", "/risk/assess", "assess_risk");
        risk_engine.endpoint("GET", "/risk/health", "risk_health");
        server.service(risk_engine);
        server.compile();
    }
}
