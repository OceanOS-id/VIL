// 013-basic-nats-worker — Java SDK equivalent
// Compile: vil compile --from java --input 013-basic-nats-worker/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("nats-worker", 8080);
        ServiceProcess nats = new ServiceProcess("nats");
        nats.endpoint("GET", "/nats/config", "nats_config");
        nats.endpoint("POST", "/nats/publish", "nats_publish");
        nats.endpoint("GET", "/nats/jetstream", "jetstream_info");
        nats.endpoint("GET", "/nats/kv", "kv_demo");
        server.service(nats);
        ServiceProcess root = new ServiceProcess("root");
        server.service(root);
        server.compile();
    }
}
