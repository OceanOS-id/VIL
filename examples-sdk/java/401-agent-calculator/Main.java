// 401-agent-calculator — Java SDK equivalent
// Compile: vil compile --from java --input 401-agent-calculator/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("calculator-agent", 3120);
        ServiceProcess calc_agent = new ServiceProcess("calc-agent");
        calc_agent.endpoint("POST", "/calc", "calc_handler");
        server.service(calc_agent);
        server.compile();
    }
}
