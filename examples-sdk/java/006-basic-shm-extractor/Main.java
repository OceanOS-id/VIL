// 006-basic-shm-extractor — Java SDK equivalent
// Compile: vil compile --from java --input 006-basic-shm-extractor/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("shm-extractor-demo", 8080);
        ServiceProcess shm_demo = new ServiceProcess("shm-demo");
        shm_demo.endpoint("POST", "/ingest", "ingest");
        shm_demo.endpoint("POST", "/compute", "compute");
        shm_demo.endpoint("GET", "/shm-stats", "shm_stats");
        shm_demo.endpoint("GET", "/benchmark", "benchmark");
        server.service(shm_demo);
        server.compile();
    }
}
