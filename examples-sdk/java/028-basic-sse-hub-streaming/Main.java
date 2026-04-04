// 028-basic-sse-hub-streaming — Java SDK equivalent
// Compile: vil compile --from java --input 028-basic-sse-hub-streaming/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("sse-hub-demo", 8080);
        ServiceProcess events = new ServiceProcess("events");
        events.endpoint("POST", "/publish", "publish");
        events.endpoint("GET", "/stream", "stream");
        events.endpoint("GET", "/stats", "stats");
        server.service(events);
        server.compile();
    }
}
