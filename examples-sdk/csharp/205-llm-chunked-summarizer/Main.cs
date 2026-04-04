// 205-llm-chunked-summarizer — C# SDK equivalent
var p = new VilPipeline("ChunkedSummarizerPipeline", 8080);
p.Route("sink.trigger_out", "source_summarize.trigger_in", "LoanWrite");
p.Route("source_summarize.response_data_out", "sink.response_data_in", "LoanWrite");
p.Route("source_summarize.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.Compile();
