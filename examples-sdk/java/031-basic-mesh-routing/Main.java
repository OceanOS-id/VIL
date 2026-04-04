// 031-basic-mesh-routing — Java SDK equivalent
// Compile: vil compile --from java --input 031-basic-mesh-routing/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("banking-transaction-mesh", 8080);
        ServiceProcess teller = new ServiceProcess("teller");
        teller.endpoint("GET", "/ping", "teller_ping");
        teller.endpoint("POST", "/submit", "teller_submit");
        server.service(teller);
        ServiceProcess fraud_check = new ServiceProcess("fraud_check");
        fraud_check.endpoint("POST", "/analyze", "fraud_process");
        server.service(fraud_check);
        ServiceProcess core_banking = new ServiceProcess("core_banking");
        core_banking.endpoint("POST", "/post", "core_banking_post");
        server.service(core_banking);
        ServiceProcess notification = new ServiceProcess("notification");
        notification.endpoint("GET", "/send", "notification_send");
        server.service(notification);
        server.compile();
    }
}
