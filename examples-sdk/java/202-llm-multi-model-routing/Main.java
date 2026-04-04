// 202-llm-multi-model-routing — Java SDK equivalent
// Compile: vil compile --from java --input 202-llm-multi-model-routing/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("MultiModelPipeline_GPT4", 8080);
        server.compile();
    }
}
