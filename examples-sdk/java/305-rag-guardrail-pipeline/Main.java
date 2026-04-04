// 305-rag-guardrail-pipeline — Java SDK equivalent
// Compile: vil compile --from java --input 305-rag-guardrail-pipeline/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("rag-guardrail-pipeline", 3114);
        ServiceProcess rag_guardrail = new ServiceProcess("rag-guardrail");
        rag_guardrail.endpoint("POST", "/safe-rag", "safe_rag_handler");
        server.service(rag_guardrail);
        server.compile();
    }
}
