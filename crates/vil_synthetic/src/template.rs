// ── N02: Generation Templates ───────────────────────────────────────
use serde::{Deserialize, Serialize};

/// A template for generating synthetic training examples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTemplate {
    pub name: String,
    pub instruction_template: String,
    pub input_template: String,
    pub output_template: String,
}

impl GenerationTemplate {
    pub fn new(
        name: impl Into<String>,
        instruction_template: impl Into<String>,
        input_template: impl Into<String>,
        output_template: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            instruction_template: instruction_template.into(),
            input_template: input_template.into(),
            output_template: output_template.into(),
        }
    }

    /// Render the template with the given seed values.
    pub fn render(&self, seed_instruction: &str, seed_input: &str, seed_output: &str) -> RenderedExample {
        RenderedExample {
            instruction: self.instruction_template.replace("{seed_instruction}", seed_instruction),
            input: self.input_template.replace("{seed_input}", seed_input),
            output: self.output_template.replace("{seed_output}", seed_output),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderedExample {
    pub instruction: String,
    pub input: String,
    pub output: String,
}

// ── Pre-built templates ─────────────────────────────────────────────

/// Q&A generation template.
pub fn qa_template() -> GenerationTemplate {
    GenerationTemplate::new(
        "qa",
        "Answer the following question: {seed_instruction}",
        "{seed_input}",
        "{seed_output}",
    )
}

/// Instruction-following generation template.
pub fn instruction_template() -> GenerationTemplate {
    GenerationTemplate::new(
        "instruction",
        "Follow the instruction: {seed_instruction}",
        "{seed_input}",
        "{seed_output}",
    )
}

/// Multi-turn conversation template.
pub fn conversation_template() -> GenerationTemplate {
    GenerationTemplate::new(
        "conversation",
        "Continue the conversation. {seed_instruction}",
        "User: {seed_input}",
        "Assistant: {seed_output}",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_qa_template() {
        let t = qa_template();
        let r = t.render("What is Rust?", "", "Rust is a systems programming language.");
        assert!(r.instruction.contains("What is Rust?"));
        assert!(r.output.contains("systems programming"));
    }

    #[test]
    fn render_conversation_template() {
        let t = conversation_template();
        let r = t.render("topic: weather", "How is the weather?", "It is sunny today.");
        assert!(r.input.starts_with("User:"));
        assert!(r.output.starts_with("Assistant:"));
    }
}
