// 304-rag-citation-extraction — Java SDK equivalent
// Compile: vil compile --from java --input 304-rag-citation-extraction/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("rag-citation-extraction", 3113);
        ServiceProcess rag_citation = new ServiceProcess("rag-citation");
        rag_citation.endpoint("POST", "/cited-rag", "cited_rag_handler");
        server.service(rag_citation);
        server.compile();
    }
}
