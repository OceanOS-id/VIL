use vil_llm::{ChatMessage, ChatResponse, LlmProvider};
use vil_llm::message::LlmError;
use std::sync::Arc;
use vil_log::app_log;

/// Result of verifying draft tokens against the target model.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Number of draft tokens accepted (matching prefix length).
    pub accepted: usize,
    /// The target model's full response content for this verification call.
    pub target_content: String,
}

/// Verify draft tokens against a target (large) model.
///
/// The verification works by appending the draft tokens to the conversation
/// as an assistant message, then asking the target model to generate from the
/// same context. We compare the target's output with the draft to find the
/// longest matching prefix.
pub async fn verify_draft(
    target: &Arc<dyn LlmProvider>,
    messages: &[ChatMessage],
    draft_tokens: &[String],
) -> Result<VerificationResult, LlmError> {
    if draft_tokens.is_empty() {
        return Ok(VerificationResult {
            accepted: 0,
            target_content: String::new(),
        });
    }

    // Build the draft text from candidate tokens.
    let draft_text: String = draft_tokens.join("");

    // Ask the target model to generate given the same context.
    // We include the draft as a hint (assistant prefix) so the target can
    // verify or diverge.
    let mut verify_messages = messages.to_vec();
    verify_messages.push(ChatMessage::assistant(&draft_text));

    let response: ChatResponse = target.chat(&verify_messages).await?;
    let target_content = response.content;

    // Count how many draft tokens match the target's output prefix.
    let accepted = count_matching_prefix(draft_tokens, &target_content);

    app_log!(Debug, "speculative_verify", { draft_count: draft_tokens.len(), accepted: accepted });

    Ok(VerificationResult {
        accepted,
        target_content,
    })
}

/// Count how many draft tokens form a matching prefix of the target text.
fn count_matching_prefix(draft_tokens: &[String], target_text: &str) -> usize {
    let mut pos = 0;
    let mut accepted = 0;

    for token in draft_tokens {
        if pos + token.len() > target_text.len() {
            break;
        }
        if &target_text[pos..pos + token.len()] == token.as_str() {
            pos += token.len();
            accepted += 1;
        } else {
            break;
        }
    }

    accepted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matching_prefix_all_match() {
        let tokens = vec!["Hello".into(), " world".into(), "!".into()];
        assert_eq!(count_matching_prefix(&tokens, "Hello world!"), 3);
    }

    #[test]
    fn test_matching_prefix_partial() {
        let tokens = vec!["Hello".into(), " world".into(), "!".into()];
        assert_eq!(count_matching_prefix(&tokens, "Hello xyz"), 1);
    }

    #[test]
    fn test_matching_prefix_none() {
        let tokens = vec!["Hello".into()];
        assert_eq!(count_matching_prefix(&tokens, "Goodbye"), 0);
    }

    #[test]
    fn test_matching_prefix_empty() {
        let tokens: Vec<String> = vec![];
        assert_eq!(count_matching_prefix(&tokens, "anything"), 0);
    }
}
