// 039-basic-observer-dashboard — Java SDK equivalent
// Compile: vil compile --from java --input 039-basic-observer-dashboard/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("observer-demo", 8080);
        ServiceProcess demo = new ServiceProcess("demo");
        demo.endpoint("GET", "/hello", "hello");
        demo.endpoint("POST", "/echo", "echo");
        server.service(demo);
        server.compile();
    }
}
