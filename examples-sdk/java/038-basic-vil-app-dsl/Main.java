// 038-basic-vil-app-dsl — Java SDK equivalent
// Compile: vil compile --from java --input 038-basic-vil-app-dsl/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("app", 8080);
        server.compile();
    }
}
