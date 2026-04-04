// 602-db-mongo-crud — Java SDK equivalent
// Compile: vil compile --from java --input 602-db-mongo-crud/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("app", 8080);
        server.compile();
    }
}
