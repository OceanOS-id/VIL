use std::collections::HashMap;
use std::fmt;

/// Errors that can occur during prompt template operations.
#[derive(Debug, Clone, PartialEq)]
pub enum PromptError {
    /// A required variable was not provided in the render context.
    MissingVariable(String),
    /// The template string is empty.
    EmptyTemplate,
}

impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PromptError::MissingVariable(var) => write!(f, "missing variable: {var}"),
            PromptError::EmptyTemplate => write!(f, "template is empty"),
        }
    }
}

impl std::error::Error for PromptError {}

/// A compile-time validated prompt template.
///
/// Detects `{variable}` placeholders at construction time and validates that
/// all required variables are provided when rendering.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromptTemplate {
    /// The raw template string containing `{variable}` placeholders.
    pub template: String,
    /// The list of variable names extracted from the template.
    pub variables: Vec<String>,
}

impl PromptTemplate {
    /// Create a new `PromptTemplate` by parsing `{variable}` placeholders.
    ///
    /// Returns `Err(PromptError::EmptyTemplate)` if the template is empty.
    pub fn new(template: &str) -> Result<Self, PromptError> {
        if template.is_empty() {
            return Err(PromptError::EmptyTemplate);
        }

        let variables = Self::extract_variables(template);

        Ok(Self {
            template: template.to_string(),
            variables,
        })
    }

    /// Render the template by replacing all `{variable}` placeholders with
    /// values from `vars`.
    ///
    /// Returns `Err(PromptError::MissingVariable)` if any required variable
    /// is not present in `vars`.
    pub fn render(&self, vars: &HashMap<String, String>) -> Result<String, PromptError> {
        let mut result = self.template.clone();

        for var in &self.variables {
            let placeholder = format!("{{{var}}}");
            match vars.get(var) {
                Some(value) => {
                    result = result.replace(&placeholder, value);
                }
                None => {
                    return Err(PromptError::MissingVariable(var.clone()));
                }
            }
        }

        Ok(result)
    }

    /// Extract variable names from `{...}` placeholders.
    fn extract_variables(template: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let chars: Vec<char> = template.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            if chars[i] == '{' {
                if let Some(end) = chars[i + 1..].iter().position(|&c| c == '}') {
                    let var_name: String = chars[i + 1..i + 1 + end].iter().collect();
                    let trimmed = var_name.trim().to_string();
                    if !trimmed.is_empty() && seen.insert(trimmed.clone()) {
                        vars.push(trimmed);
                    }
                    i = i + 1 + end + 1;
                    continue;
                }
            }
            i += 1;
        }

        vars
    }

    /// Returns the number of unique variables in this template.
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_render() {
        let tpl = PromptTemplate::new("Hello, {name}!").unwrap();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());
        assert_eq!(tpl.render(&vars).unwrap(), "Hello, World!");
    }

    #[test]
    fn missing_variable_error() {
        let tpl = PromptTemplate::new("Hello, {name}!").unwrap();
        let vars = HashMap::new();
        let err = tpl.render(&vars).unwrap_err();
        assert_eq!(err, PromptError::MissingVariable("name".to_string()));
    }

    #[test]
    fn multiple_variables() {
        let tpl = PromptTemplate::new("{greeting}, {name}! Welcome to {place}.").unwrap();
        assert_eq!(tpl.variables.len(), 3);
        let mut vars = HashMap::new();
        vars.insert("greeting".to_string(), "Hi".to_string());
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("place".to_string(), "VIL".to_string());
        assert_eq!(tpl.render(&vars).unwrap(), "Hi, Alice! Welcome to VIL.");
    }

    #[test]
    fn empty_template_error() {
        let err = PromptTemplate::new("").unwrap_err();
        assert_eq!(err, PromptError::EmptyTemplate);
    }

    #[test]
    fn no_variables_template() {
        let tpl = PromptTemplate::new("No variables here.").unwrap();
        assert!(tpl.variables.is_empty());
        let vars = HashMap::new();
        assert_eq!(tpl.render(&vars).unwrap(), "No variables here.");
    }

    #[test]
    fn duplicate_variables_deduplicated() {
        let tpl = PromptTemplate::new("{x} and {x} again").unwrap();
        assert_eq!(tpl.variables.len(), 1);
        assert_eq!(tpl.variables[0], "x");

        let mut vars = HashMap::new();
        vars.insert("x".to_string(), "val".to_string());
        assert_eq!(tpl.render(&vars).unwrap(), "val and val again");
    }

    #[test]
    fn variable_count() {
        let tpl = PromptTemplate::new("{a} {b} {c}").unwrap();
        assert_eq!(tpl.variable_count(), 3);
    }

    #[test]
    fn serde_roundtrip() {
        let tpl = PromptTemplate::new("Test {var}").unwrap();
        let json = serde_json::to_string(&tpl).unwrap();
        let restored: PromptTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.template, tpl.template);
        assert_eq!(restored.variables, tpl.variables);
    }
}
