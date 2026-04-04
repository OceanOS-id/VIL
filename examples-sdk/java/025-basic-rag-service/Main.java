// 025-basic-rag-service — Java SDK equivalent
// Compile: vil compile --from java --input 025-basic-rag-service/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("rag-service", 3091);
        ServiceProcess rag = new ServiceProcess("rag");
        rag.endpoint("POST", "/rag", "rag_handler");
        server.service(rag);
        server.compile();
    }
}
