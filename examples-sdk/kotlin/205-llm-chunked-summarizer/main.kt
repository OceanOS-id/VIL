// 205-llm-chunked-summarizer — Kotlin SDK equivalent
fun main() {
    val p = VilPipeline("ChunkedSummarizerPipeline", 8080)
    p.route("sink.trigger_out", "source_summarize.trigger_in", "LoanWrite")
    p.route("source_summarize.response_data_out", "sink.response_data_in", "LoanWrite")
    p.route("source_summarize.response_ctrl_out", "sink.response_ctrl_in", "Copy")
    p.compile()
}
