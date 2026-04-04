// 021-basic-wasm-faas — Java SDK equivalent
// Compile: vil compile --from java --input 021-basic-wasm-faas/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("wasm-faas-example", 8080);
        ServiceProcess wasm_faas = new ServiceProcess("wasm-faas");
        wasm_faas.endpoint("GET", "/", "index");
        wasm_faas.endpoint("GET", "/wasm/modules", "list_modules");
        wasm_faas.endpoint("POST", "/wasm/pricing", "invoke_pricing");
        wasm_faas.endpoint("POST", "/wasm/validation", "invoke_validation");
        wasm_faas.endpoint("POST", "/wasm/transform", "invoke_transform");
        server.service(wasm_faas);
        server.compile();
    }
}
