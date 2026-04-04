// 202-llm-multi-model-routing — Java SDK equivalent
import dev.vil.*;
public class Main {
    public static void main(String[] args) {
        VilPipeline p = new VilPipeline("MultiModelPipeline_GPT4", 8080);
        p.route("sink.trigger_out", "source_gpt4.trigger_in", "LoanWrite");
        p.route("source_gpt4.response_data_out", "sink.response_data_in", "LoanWrite");
        p.route("source_gpt4.response_ctrl_out", "sink.response_ctrl_in", "Copy");
        p.compile();
    }
}
