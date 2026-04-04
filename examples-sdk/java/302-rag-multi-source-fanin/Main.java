// 302-rag-multi-source-fanin — Java SDK equivalent
import dev.vil.*;
public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("rag-multi-source-fanin", 3111);
        p.compile();
    }
}
