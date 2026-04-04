// 302-rag-multi-source-fanin — Java SDK equivalent
// Compile: vil compile --from java --input 302-rag-multi-source-fanin/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("rag-multi-source-fanin", 3111);
        server.compile();
    }
}
