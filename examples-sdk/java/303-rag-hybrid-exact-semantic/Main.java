// 303-rag-hybrid-exact-semantic — Java SDK equivalent
// Compile: vil compile --from java --input 303-rag-hybrid-exact-semantic/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("rag-hybrid-exact-semantic", 3112);
        ServiceProcess rag_hybrid = new ServiceProcess("rag-hybrid");
        rag_hybrid.endpoint("POST", "/hybrid", "hybrid_handler");
        server.service(rag_hybrid);
        server.compile();
    }
}
