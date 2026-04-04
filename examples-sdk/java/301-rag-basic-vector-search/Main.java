// 301-rag-basic-vector-search — Java SDK equivalent
// Compile: vil compile --from java --input 301-rag-basic-vector-search/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("rag-basic-vector-search", 3110);
        ServiceProcess rag_basic = new ServiceProcess("rag-basic");
        rag_basic.endpoint("POST", "/rag", "rag_handler");
        server.service(rag_basic);
        server.compile();
    }
}
