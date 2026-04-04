// 101c-vilapp-multi-pipeline-benchmark — Java SDK equivalent
// Compile: vil compile --from java --input 101c-vilapp-multi-pipeline-benchmark/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("multi-pipeline-bench", 3090);
        ServiceProcess pipeline = new ServiceProcess("pipeline");
        server.service(pipeline);
        server.compile();
    }
}
