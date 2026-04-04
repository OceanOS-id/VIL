// 037-basic-vilmodel-derive — Java SDK equivalent
// Compile: vil compile --from java --input 037-basic-vilmodel-derive/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("insurance-claim-processing", 8080);
        ServiceProcess claims = new ServiceProcess("claims");
        claims.endpoint("POST", "/claims/submit", "submit_claim");
        claims.endpoint("GET", "/claims/sample", "sample_claim");
        server.service(claims);
        server.compile();
    }
}
