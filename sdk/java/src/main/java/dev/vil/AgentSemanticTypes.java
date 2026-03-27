package dev.vil;

/**
 * Agent plugin semantic types for VIL Phase 6.
 *
 * <p>Mirror of Rust vil_agent::semantic types for Java SDK interop.
 */
public class AgentSemanticTypes {

    /** Agent task completion event. */
    public static class AgentCompletionEvent {
        public String agentId;
        public String taskResult;
        public String[] toolsUsed;
        public int stepCount;

        public AgentCompletionEvent() {}

        public AgentCompletionEvent(String agentId, String taskResult, String[] toolsUsed, int stepCount) {
            this.agentId = agentId;
            this.taskResult = taskResult;
            this.toolsUsed = toolsUsed;
            this.stepCount = stepCount;
        }
    }

    /** Agent fault (tool error, timeout, etc.). */
    public static class AgentFault {
        public String code;
        public String message;
        public String toolName;
        public boolean recoverable;

        public AgentFault() {}

        public AgentFault(String code, String message, String toolName, boolean recoverable) {
            this.code = code;
            this.message = message;
            this.toolName = toolName;
            this.recoverable = recoverable;
        }
    }

    /** Agent memory/context state. */
    public static class AgentMemoryState {
        public long entryCount;
        public long contextTokens;

        public AgentMemoryState() {}

        public AgentMemoryState(long entryCount, long contextTokens) {
            this.entryCount = entryCount;
            this.contextTokens = contextTokens;
        }
    }
}
