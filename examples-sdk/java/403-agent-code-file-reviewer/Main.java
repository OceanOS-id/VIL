// 403-agent-code-file-reviewer — Java SDK equivalent
// Compile: vil compile --from java --input 403-agent-code-file-reviewer/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("code-file-reviewer-agent", 3122);
        ServiceProcess code_review_agent = new ServiceProcess("code-review-agent");
        code_review_agent.endpoint("POST", "/code-review", "code_review_handler");
        server.service(code_review_agent);
        server.compile();
    }
}
