use std::collections::HashMap;

use crate::template::{PromptError, PromptTemplate};

/// A named store of prompt templates.
///
/// Allows registering, retrieving, and rendering templates by name.
#[derive(Debug, Default)]
pub struct PromptRegistry {
    templates: HashMap<String, PromptTemplate>,
}

impl PromptRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Register a template under the given name, replacing any existing template
    /// with the same name.
    pub fn register(&mut self, name: &str, template: PromptTemplate) {
        self.templates.insert(name.to_string(), template);
    }

    /// Get a reference to the template registered under `name`.
    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    /// Render the template registered under `name` with the given variables.
    ///
    /// Returns `None` if the template is not found.
    pub fn render(
        &self,
        name: &str,
        vars: &HashMap<String, String>,
    ) -> Option<Result<String, PromptError>> {
        self.templates.get(name).map(|tpl| tpl.render(vars))
    }

    /// Remove a template by name. Returns `true` if it existed.
    pub fn remove(&mut self, name: &str) -> bool {
        self.templates.remove(name).is_some()
    }

    /// Returns the number of registered templates.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Returns `true` if the registry has no templates.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Returns an iterator over template names.
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.templates.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_register_and_get() {
        let mut reg = PromptRegistry::new();
        let tpl = PromptTemplate::new("Hello {name}").unwrap();
        reg.register("greet", tpl);
        assert!(reg.get("greet").is_some());
        assert!(reg.get("missing").is_none());
    }

    #[test]
    fn registry_render() {
        let mut reg = PromptRegistry::new();
        let tpl = PromptTemplate::new("Hi {name}").unwrap();
        reg.register("greet", tpl);

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Bob".to_string());
        let result = reg.render("greet", &vars).unwrap().unwrap();
        assert_eq!(result, "Hi Bob");
    }

    #[test]
    fn registry_render_missing_template() {
        let reg = PromptRegistry::new();
        let vars = HashMap::new();
        assert!(reg.render("nope", &vars).is_none());
    }

    #[test]
    fn registry_remove() {
        let mut reg = PromptRegistry::new();
        let tpl = PromptTemplate::new("test").unwrap();
        reg.register("t", tpl);
        assert_eq!(reg.len(), 1);
        assert!(reg.remove("t"));
        assert_eq!(reg.len(), 0);
        assert!(!reg.remove("t"));
    }

    #[test]
    fn registry_len_and_empty() {
        let mut reg = PromptRegistry::new();
        assert!(reg.is_empty());
        reg.register("a", PromptTemplate::new("x").unwrap());
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
    }
}
