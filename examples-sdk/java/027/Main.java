// 027 — VilServer Minimal (No VX)
// Equivalent to: examples/027-basic-vilserver-minimal (Rust)
// Compile: vil compile --from java --input 027/Main.java --release
package dev.vil.examples;

import dev.vil.VilServer;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("minimal-api", 8080);

        // -- Fault type -------------------------------------------------------
        server.fault("ApiFault", List.of("InvalidInput", "NotFound"));

        // -- Routes (no ServiceProcess, no VX) --------------------------------
        server.route("GET", "/hello", "hello");
        server.route("POST", "/echo", "echo");

        // Built-in: GET /health, /ready, /metrics, /info

        // -- Emit / compile ---------------------------------------------------
        if ("manifest".equals(System.getenv("VIL_COMPILE_MODE"))) {
            System.out.println(server.toYaml());
        } else {
            server.compile();
        }
    }
}
