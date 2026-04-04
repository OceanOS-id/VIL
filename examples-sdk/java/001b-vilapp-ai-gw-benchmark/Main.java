// 001b-vilapp-ai-gw-benchmark — Java SDK equivalent
// Compile: vil compile --from java --input 001b-vilapp-ai-gw-benchmark/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("ai-gw-bench", 3081);
        ServiceProcess gw = new ServiceProcess("gw");
        server.service(gw);
        server.compile();
    }
}
