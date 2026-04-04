// 501-villog-stdout-dev — Java SDK equivalent
// Compile: vil compile --from java --input 501-villog-stdout-dev/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("app", 8080);
        server.compile();
    }
}
