// 030-basic-trilane-messaging — Java SDK equivalent
// Compile: vil compile --from java --input 030-basic-trilane-messaging/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("ecommerce-order-pipeline", 8080);
        ServiceProcess gateway = new ServiceProcess("gateway");
        server.service(gateway);
        ServiceProcess fulfillment = new ServiceProcess("fulfillment");
        fulfillment.endpoint("GET", "/status", "fulfillment_status");
        server.service(fulfillment);
        server.compile();
    }
}
