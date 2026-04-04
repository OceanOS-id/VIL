// 205-llm-chunked-summarizer — Java SDK equivalent
// Compile: vil compile --from java --input 205-llm-chunked-summarizer/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("ChunkedSummarizerPipeline", 8080);
        server.compile();
    }
}
