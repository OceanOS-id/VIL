package dev.vil;

/**
 * LLM plugin semantic types for VIL Phase 6.
 *
 * <p>Mirror of Rust vil_llm::semantic types for Java SDK interop.
 */
public class LlmSemanticTypes {

    /** LLM response completion event. */
    public static class LlmResponseEvent {
        public String model;
        public String content;
        public String finishReason;
        public int tokensUsed;

        public LlmResponseEvent() {}

        public LlmResponseEvent(String model, String content, String finishReason, int tokensUsed) {
            this.model = model;
            this.content = content;
            this.finishReason = finishReason;
            this.tokensUsed = tokensUsed;
        }
    }

    /** LLM fault (provider error, rate limit, etc.). */
    public static class LlmFault {
        public String code;
        public String message;
        public String provider;
        public boolean retryable;

        public LlmFault() {}

        public LlmFault(String code, String message, String provider, boolean retryable) {
            this.code = code;
            this.message = message;
            this.provider = provider;
            this.retryable = retryable;
        }
    }

    /** LLM usage tracking state. */
    public static class LlmUsageState {
        public long totalTokens;
        public double totalCost;
        public long requestCount;

        public LlmUsageState() {}

        public LlmUsageState(long totalTokens, double totalCost, long requestCount) {
            this.totalTokens = totalTokens;
            this.totalCost = totalCost;
            this.requestCount = requestCount;
        }
    }
}
