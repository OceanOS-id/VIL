//! ResourceRegistry -- typed dependency injection for plugin-to-plugin resource sharing.
//!
//! Plugins provide resources during registration:
//!   ctx.provide::<Arc<dyn LlmProvider>>("openai", provider);
//!
//! Other plugins consume them:
//!   let llm = ctx.require::<Arc<dyn LlmProvider>>("openai");

use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;

/// Type-erased resource registry keyed by (TypeId, name).
///
/// Same type can have multiple named instances
/// (e.g., "openai" and "ollama" both as dyn LlmProvider).
pub struct ResourceRegistry {
    resources: HashMap<(TypeId, String), Box<dyn Any + Send + Sync>>,
    type_names: HashMap<(TypeId, String), &'static str>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            type_names: HashMap::new(),
        }
    }

    /// Register a typed resource with a name.
    pub fn provide<T: Send + Sync + 'static>(&mut self, name: &str, resource: T) {
        let key = (TypeId::of::<T>(), name.to_string());
        self.type_names.insert(key.clone(), type_name::<T>());
        self.resources.insert(key, Box::new(resource));
    }

    /// Get a typed resource by name. Returns None if not found.
    pub fn get<T: Send + Sync + 'static>(&self, name: &str) -> Option<&T> {
        let key = (TypeId::of::<T>(), name.to_string());
        self.resources.get(&key)?.downcast_ref::<T>()
    }

    /// Get a typed resource, panic with helpful message if not found.
    pub fn require<T: Send + Sync + 'static>(&self, name: &str) -> &T {
        self.get::<T>(name).unwrap_or_else(|| {
            panic!(
                "Plugin resource not found: {}(\"{}\"). Ensure the providing plugin is registered before this one.",
                type_name::<T>(), name
            )
        })
    }

    /// Check if a resource exists.
    pub fn has<T: Send + Sync + 'static>(&self, name: &str) -> bool {
        let key = (TypeId::of::<T>(), name.to_string());
        self.resources.contains_key(&key)
    }

    /// List all registered resource keys as (type_name, resource_name).
    pub fn list(&self) -> Vec<(&'static str, &str)> {
        self.type_names
            .iter()
            .map(|((_, name), type_name)| (*type_name, name.as_str()))
            .collect()
    }

    /// Total number of registered resources.
    pub fn count(&self) -> usize {
        self.resources.len()
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    trait MyTrait: Send + Sync {
        fn name(&self) -> &str;
    }

    struct ImplA;
    impl MyTrait for ImplA {
        fn name(&self) -> &str {
            "A"
        }
    }

    struct ImplB;
    impl MyTrait for ImplB {
        fn name(&self) -> &str {
            "B"
        }
    }

    #[test]
    fn test_provide_and_get() {
        let mut reg = ResourceRegistry::new();
        reg.provide::<String>("greeting", "hello".to_string());
        assert_eq!(reg.get::<String>("greeting"), Some(&"hello".to_string()));
        assert_eq!(reg.get::<String>("missing"), None);
    }

    #[test]
    fn test_multiple_instances_same_type() {
        let mut reg = ResourceRegistry::new();
        let a: Arc<dyn MyTrait> = Arc::new(ImplA);
        let b: Arc<dyn MyTrait> = Arc::new(ImplB);
        reg.provide::<Arc<dyn MyTrait>>("provider-a", a);
        reg.provide::<Arc<dyn MyTrait>>("provider-b", b);

        assert_eq!(reg.require::<Arc<dyn MyTrait>>("provider-a").name(), "A");
        assert_eq!(reg.require::<Arc<dyn MyTrait>>("provider-b").name(), "B");
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn test_has() {
        let mut reg = ResourceRegistry::new();
        reg.provide::<u32>("count", 42);
        assert!(reg.has::<u32>("count"));
        assert!(!reg.has::<u32>("missing"));
        assert!(!reg.has::<String>("count")); // wrong type
    }

    #[test]
    #[should_panic(expected = "Plugin resource not found")]
    fn test_require_missing_panics() {
        let reg = ResourceRegistry::new();
        reg.require::<String>("missing");
    }

    #[test]
    fn test_list() {
        let mut reg = ResourceRegistry::new();
        reg.provide::<String>("a", "hello".into());
        reg.provide::<u32>("b", 42);
        let list = reg.list();
        assert_eq!(list.len(), 2);
    }
}
