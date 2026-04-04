// 023-basic-hybrid-wasm-sidecar — Java SDK equivalent
// Compile: vil compile --from java --input 023-basic-hybrid-wasm-sidecar/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("hybrid-pipeline", 8080);
        ServiceProcess pipeline = new ServiceProcess("pipeline");
        pipeline.endpoint("GET", "/", "index");
        pipeline.endpoint("POST", "/validate", "validate_order");
        pipeline.endpoint("POST", "/price", "calculate_price");
        pipeline.endpoint("POST", "/fraud", "fraud_check");
        pipeline.endpoint("POST", "/order", "process_order");
        server.service(pipeline);
        server.compile();
    }
}
