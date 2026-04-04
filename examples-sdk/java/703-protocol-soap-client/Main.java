// 703-protocol-soap-client — Java SDK equivalent
// Compile: vil compile --from java --input 703-protocol-soap-client/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("app", 8080);
        server.compile();
    }
}
