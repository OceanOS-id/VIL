// 402-agent-http-researcher — Java SDK equivalent
// Compile: vil compile --from java --input 402-agent-http-researcher/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("http-researcher-agent", 3121);
        ServiceProcess research_agent = new ServiceProcess("research-agent");
        research_agent.endpoint("POST", "/research", "research_handler");
        research_agent.endpoint("GET", "/products", "products_handler");
        server.service(research_agent);
        server.compile();
    }
}
