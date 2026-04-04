// 206-llm-decision-routing — Java SDK equivalent
// Compile: vil compile --from java --input 206-llm-decision-routing/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("insurance-underwriting-ai", 3116);
        ServiceProcess underwriter = new ServiceProcess("underwriter");
        server.service(underwriter);
        server.compile();
    }
}
