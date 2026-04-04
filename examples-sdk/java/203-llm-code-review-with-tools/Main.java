// 203-llm-code-review-with-tools — Java SDK equivalent
// Compile: vil compile --from java --input 203-llm-code-review-with-tools/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("llm-code-review-tools", 3102);
        ServiceProcess code_review = new ServiceProcess("code-review");
        code_review.endpoint("POST", "/code/review", "code_review_handler");
        server.service(code_review);
        server.compile();
    }
}
