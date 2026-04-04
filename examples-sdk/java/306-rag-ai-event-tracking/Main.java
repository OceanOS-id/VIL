// 306-rag-ai-event-tracking — Java SDK equivalent
// Compile: vil compile --from java --input 306-rag-ai-event-tracking/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("customer-support-rag", 3116);
        ServiceProcess support = new ServiceProcess("support");
        support.endpoint("POST", "/support/ask", "answer_question");
        server.service(support);
        server.compile();
    }
}
