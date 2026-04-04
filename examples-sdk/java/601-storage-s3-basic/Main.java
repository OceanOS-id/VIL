// 601-storage-s3-basic — Java SDK equivalent
// Compile: vil compile --from java --input 601-storage-s3-basic/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("app", 8080);
        server.compile();
    }
}
