// 026-basic-ai-agent — Java SDK equivalent
// Compile: vil compile --from java --input 026-basic-ai-agent/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("ai-agent", 8080);
        ServiceProcess agent = new ServiceProcess("agent");
        agent.endpoint("POST", "/agent", "agent_handler");
        server.service(agent);
        server.compile();
    }
}
