use crate::template::{PromptError, PromptTemplate};

/// Fluent builder for constructing prompt templates from system/user/context parts.
///
/// # Example
///
/// ```rust
/// use vil_prompts::PromptBuilder;
///
/// let tpl = PromptBuilder::new()
///     .system("You are a helpful assistant.")
///     .user("{question}")
///     .context("{context}")
///     .build()
///     .unwrap();
///
/// assert_eq!(tpl.variables.len(), 2);
/// ```
#[derive(Debug, Default)]
pub struct PromptBuilder {
    system: Option<String>,
    user: Option<String>,
    context: Option<String>,
    custom_sections: Vec<(String, String)>,
}

impl PromptBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the system instruction section.
    pub fn system(mut self, system: &str) -> Self {
        self.system = Some(system.to_string());
        self
    }

    /// Set the user message section.
    pub fn user(mut self, user: &str) -> Self {
        self.user = Some(user.to_string());
        self
    }

    /// Set the context section.
    pub fn context(mut self, context: &str) -> Self {
        self.context = Some(context.to_string());
        self
    }

    /// Add a custom named section to the prompt.
    pub fn section(mut self, name: &str, content: &str) -> Self {
        self.custom_sections
            .push((name.to_string(), content.to_string()));
        self
    }

    /// Build the final `PromptTemplate` from the configured sections.
    ///
    /// Returns `Err(PromptError::EmptyTemplate)` if no sections were added.
    pub fn build(self) -> Result<PromptTemplate, PromptError> {
        let mut parts = Vec::new();

        if let Some(sys) = &self.system {
            parts.push(format!("[System]\n{sys}"));
        }
        if let Some(ctx) = &self.context {
            parts.push(format!("[Context]\n{ctx}"));
        }
        for (name, content) in &self.custom_sections {
            parts.push(format!("[{name}]\n{content}"));
        }
        if let Some(usr) = &self.user {
            parts.push(format!("[User]\n{usr}"));
        }

        if parts.is_empty() {
            return Err(PromptError::EmptyTemplate);
        }

        let combined = parts.join("\n\n");
        PromptTemplate::new(&combined)
    }
}

// ── Pre-built Templates ─────────────────────────────────────────────────────

/// Pre-built RAG question-answering template.
///
/// Variables: `context`, `question`
pub fn rag_qa_template() -> PromptTemplate {
    PromptBuilder::new()
        .system("You are a helpful assistant. Answer the question using only the provided context. If the context does not contain enough information, say so.")
        .context("{context}")
        .user("{question}")
        .build()
        .expect("rag_qa_template is a valid template")
}

/// Pre-built summarization template.
///
/// Variables: `text`
pub fn summarize_template() -> PromptTemplate {
    PromptBuilder::new()
        .system("You are a concise summarizer. Summarize the following text in a few sentences.")
        .user("{text}")
        .build()
        .expect("summarize_template is a valid template")
}

/// Pre-built code review template.
///
/// Variables: `language`, `code`
pub fn code_review_template() -> PromptTemplate {
    PromptBuilder::new()
        .system("You are an expert code reviewer. Review the following {language} code for bugs, performance issues, and style improvements.")
        .user("{code}")
        .build()
        .expect("code_review_template is a valid template")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn builder_basic() {
        let tpl = PromptBuilder::new()
            .system("Be helpful.")
            .user("{question}")
            .build()
            .unwrap();

        assert!(tpl.variables.contains(&"question".to_string()));
    }

    #[test]
    fn builder_all_sections() {
        let tpl = PromptBuilder::new()
            .system("System prompt.")
            .context("{ctx}")
            .user("{q}")
            .build()
            .unwrap();

        assert!(tpl.variables.contains(&"ctx".to_string()));
        assert!(tpl.variables.contains(&"q".to_string()));
        assert!(tpl.template.contains("[System]"));
        assert!(tpl.template.contains("[Context]"));
        assert!(tpl.template.contains("[User]"));
    }

    #[test]
    fn builder_empty_error() {
        let err = PromptBuilder::new().build().unwrap_err();
        assert_eq!(err, PromptError::EmptyTemplate);
    }

    #[test]
    fn builder_custom_section() {
        let tpl = PromptBuilder::new()
            .system("sys")
            .section("Examples", "{examples}")
            .user("{input}")
            .build()
            .unwrap();

        assert!(tpl.template.contains("[Examples]"));
        assert!(tpl.variables.contains(&"examples".to_string()));
    }

    #[test]
    fn rag_qa_template_works() {
        let tpl = rag_qa_template();
        assert!(tpl.variables.contains(&"context".to_string()));
        assert!(tpl.variables.contains(&"question".to_string()));

        let mut vars = HashMap::new();
        vars.insert("context".to_string(), "VIL is fast.".to_string());
        vars.insert("question".to_string(), "What is VIL?".to_string());
        let rendered = tpl.render(&vars).unwrap();
        assert!(rendered.contains("VIL is fast."));
        assert!(rendered.contains("What is VIL?"));
    }

    #[test]
    fn summarize_template_works() {
        let tpl = summarize_template();
        assert!(tpl.variables.contains(&"text".to_string()));

        let mut vars = HashMap::new();
        vars.insert("text".to_string(), "Long text here.".to_string());
        let rendered = tpl.render(&vars).unwrap();
        assert!(rendered.contains("Long text here."));
    }

    #[test]
    fn code_review_template_works() {
        let tpl = code_review_template();
        assert!(tpl.variables.contains(&"language".to_string()));
        assert!(tpl.variables.contains(&"code".to_string()));

        let mut vars = HashMap::new();
        vars.insert("language".to_string(), "Rust".to_string());
        vars.insert("code".to_string(), "fn main() {}".to_string());
        let rendered = tpl.render(&vars).unwrap();
        assert!(rendered.contains("Rust"));
        assert!(rendered.contains("fn main() {}"));
    }
}
