// 032-basic-failover-ha — Java SDK equivalent
// Compile: vil compile --from java --input 032-basic-failover-ha/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("payment-gateway-ha", 8080);
        ServiceProcess primary = new ServiceProcess("primary");
        primary.endpoint("GET", "/health", "primary_health");
        primary.endpoint("POST", "/charge", "primary_charge");
        server.service(primary);
        ServiceProcess backup = new ServiceProcess("backup");
        backup.endpoint("GET", "/health", "backup_health");
        backup.endpoint("POST", "/charge", "backup_charge");
        server.service(backup);
        server.compile();
    }
}
