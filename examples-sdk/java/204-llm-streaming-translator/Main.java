// 204-llm-streaming-translator — Java SDK equivalent
// Compile: vil compile --from java --input 204-llm-streaming-translator/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("llm-streaming-translator", 3103);
        ServiceProcess translator = new ServiceProcess("translator");
        server.service(translator);
        server.compile();
    }
}
