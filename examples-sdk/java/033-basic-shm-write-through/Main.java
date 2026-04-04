// 033-basic-shm-write-through — Java SDK equivalent
// Compile: vil compile --from java --input 033-basic-shm-write-through/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("realtime-analytics-dashboard", 8080);
        ServiceProcess catalog = new ServiceProcess("catalog");
        catalog.endpoint("POST", "/catalog/search", "catalog_search");
        catalog.endpoint("GET", "/catalog/health", "catalog_health");
        server.service(catalog);
        server.compile();
    }
}
