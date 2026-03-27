// 003 — Hello Server (VX_APP)
// Equivalent to: examples/003-basic-hello-server (Rust)
// Compile: vil compile --from java --input 003/Main.java --release
package dev.vil.examples;

import dev.vil.VilServer;
import dev.vil.ServiceProcess;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("hello-server", 8080);

        // -- ServiceProcess: hello (prefix: /api/hello) -----------------------
        ServiceProcess hello = new ServiceProcess("hello");
        hello.endpoint("GET", "/", "hello");
        hello.endpoint("GET", "/greet/:name", "greet");
        hello.endpoint("POST", "/echo", "echo");
        hello.endpoint("GET", "/shm-info", "shm_info");
        server.service(hello, "/api/hello");

        // Built-in: GET /health, /ready, /metrics, /info

        // -- Emit / compile ---------------------------------------------------
        if ("manifest".equals(System.getenv("VIL_COMPILE_MODE"))) {
            System.out.println(server.toYaml());
        } else {
            server.compile();
        }
    }
}
