//! Conversation memory with sliding window for agent context management.

use vil_llm::ChatMessage;

/// Sliding-window conversation memory.
///
/// Maintains a bounded list of messages to stay within LLM context limits.
/// Optionally prepends a system prompt to every context retrieval.
pub struct ConversationMemory {
    messages: tokio::sync::RwLock<Vec<ChatMessage>>,
    max_messages: usize,
    system_prompt: Option<String>,
}

impl ConversationMemory {
    /// Create a new conversation memory with the given sliding window size.
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: tokio::sync::RwLock::new(Vec::new()),
            max_messages,
            system_prompt: None,
        }
    }

    /// Set the system prompt (builder-style).
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add a message to the conversation.
    /// If the window is full, the oldest non-system message is dropped.
    pub async fn add(&self, message: ChatMessage) {
        let mut msgs = self.messages.write().await;
        msgs.push(message);
        // Trim to sliding window
        if msgs.len() > self.max_messages {
            let excess = msgs.len() - self.max_messages;
            msgs.drain(..excess);
        }
    }

    /// Get the full context: system prompt (if any) + conversation messages.
    pub async fn get_context(&self) -> Vec<ChatMessage> {
        let msgs = self.messages.read().await;
        let mut context = Vec::with_capacity(msgs.len() + 1);
        if let Some(ref prompt) = self.system_prompt {
            context.push(ChatMessage::system(prompt.clone()));
        }
        context.extend(msgs.iter().cloned());
        context
    }

    /// Clear all messages.
    pub async fn clear(&self) {
        let mut msgs = self.messages.write().await;
        msgs.clear();
    }

    /// Number of messages currently stored (excludes system prompt).
    pub async fn len(&self) -> usize {
        let msgs = self.messages.read().await;
        msgs.len()
    }

    /// Whether the memory is empty.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_llm::Role;

    #[tokio::test]
    async fn test_add_and_get_context() {
        let mem = ConversationMemory::new(10);
        mem.add(ChatMessage::user("hello")).await;
        mem.add(ChatMessage::assistant("hi there")).await;

        let ctx = mem.get_context().await;
        assert_eq!(ctx.len(), 2);
        assert_eq!(ctx[0].content, "hello");
        assert_eq!(ctx[1].content, "hi there");
    }

    #[tokio::test]
    async fn test_system_prompt_prepended() {
        let mem = ConversationMemory::new(10).with_system_prompt("You are helpful.");
        mem.add(ChatMessage::user("test")).await;

        let ctx = mem.get_context().await;
        assert_eq!(ctx.len(), 2);
        assert!(matches!(ctx[0].role, Role::System));
        assert_eq!(ctx[0].content, "You are helpful.");
        assert_eq!(ctx[1].content, "test");
    }

    #[tokio::test]
    async fn test_sliding_window() {
        let mem = ConversationMemory::new(3);
        mem.add(ChatMessage::user("msg1")).await;
        mem.add(ChatMessage::user("msg2")).await;
        mem.add(ChatMessage::user("msg3")).await;
        mem.add(ChatMessage::user("msg4")).await;

        let ctx = mem.get_context().await;
        assert_eq!(ctx.len(), 3);
        assert_eq!(ctx[0].content, "msg2");
        assert_eq!(ctx[1].content, "msg3");
        assert_eq!(ctx[2].content, "msg4");
    }

    #[tokio::test]
    async fn test_clear() {
        let mem = ConversationMemory::new(10);
        mem.add(ChatMessage::user("hello")).await;
        assert_eq!(mem.len().await, 1);

        mem.clear().await;
        assert_eq!(mem.len().await, 0);
        assert!(mem.is_empty().await);
    }

    #[tokio::test]
    async fn test_len() {
        let mem = ConversationMemory::new(10);
        assert_eq!(mem.len().await, 0);
        mem.add(ChatMessage::user("a")).await;
        mem.add(ChatMessage::user("b")).await;
        assert_eq!(mem.len().await, 2);
    }
}
