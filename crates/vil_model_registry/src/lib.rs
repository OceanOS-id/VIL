//! # vil_model_registry (I10)
//!
//! Model Registry — versioned model management with promotion and rollback.
//!
//! Stores model versions with metadata, supports promoting versions to Active,
//! rolling back to previous versions, and deprecating old versions.

pub mod model;
pub mod registry;
pub mod version;
pub mod semantic;
pub mod handlers;
pub mod plugin;
pub mod pipeline_sse;

pub use model::{ModelEntry, ModelStatus};
pub use registry::ModelRegistry;
pub use plugin::ModelRegistryPlugin;
pub use semantic::{RegistryEvent, RegistryFault, RegistryFaultType, RegistryState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register() {
        let reg = ModelRegistry::new();
        let v = reg.register("gpt-4", "openai", serde_json::json!({"temp": 0.7}));
        assert_eq!(v, 1);
    }

    #[test]
    fn test_register_increments_version() {
        let reg = ModelRegistry::new();
        let v1 = reg.register("gpt-4", "openai", serde_json::json!({}));
        let v2 = reg.register("gpt-4", "openai", serde_json::json!({}));
        let v3 = reg.register("gpt-4", "openai", serde_json::json!({}));
        assert_eq!(v1, 1);
        assert_eq!(v2, 2);
        assert_eq!(v3, 3);
    }

    #[test]
    fn test_get_active_none_initially() {
        let reg = ModelRegistry::new();
        reg.register("gpt-4", "openai", serde_json::json!({}));
        // New registrations are Staging, not Active
        assert!(reg.get_active("gpt-4").is_none());
    }

    #[test]
    fn test_promote() {
        let reg = ModelRegistry::new();
        let v = reg.register("gpt-4", "openai", serde_json::json!({}));
        reg.promote("gpt-4", v).unwrap();
        let active = reg.get_active("gpt-4").unwrap();
        assert_eq!(active.version, v);
        assert_eq!(active.status, ModelStatus::Active);
    }

    #[test]
    fn test_promote_replaces_previous_active() {
        let reg = ModelRegistry::new();
        let v1 = reg.register("gpt-4", "openai", serde_json::json!({}));
        let v2 = reg.register("gpt-4", "openai", serde_json::json!({}));
        reg.promote("gpt-4", v1).unwrap();
        reg.promote("gpt-4", v2).unwrap();
        let active = reg.get_active("gpt-4").unwrap();
        assert_eq!(active.version, v2);
    }

    #[test]
    fn test_rollback() {
        let reg = ModelRegistry::new();
        let v1 = reg.register("gpt-4", "openai", serde_json::json!({}));
        let v2 = reg.register("gpt-4", "openai", serde_json::json!({}));
        reg.promote("gpt-4", v1).unwrap();
        reg.promote("gpt-4", v2).unwrap();
        reg.rollback("gpt-4").unwrap();
        let active = reg.get_active("gpt-4").unwrap();
        assert_eq!(active.version, v1);
    }

    #[test]
    fn test_deprecate() {
        let reg = ModelRegistry::new();
        let v = reg.register("gpt-4", "openai", serde_json::json!({}));
        reg.deprecate("gpt-4", v).unwrap();
        let history = reg.history("gpt-4");
        assert_eq!(history[0].status, ModelStatus::Deprecated);
    }

    #[test]
    fn test_version_history() {
        let reg = ModelRegistry::new();
        reg.register("gpt-4", "openai", serde_json::json!({"v": 1}));
        reg.register("gpt-4", "openai", serde_json::json!({"v": 2}));
        reg.register("gpt-4", "openai", serde_json::json!({"v": 3}));
        let history = reg.history("gpt-4");
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].version, 1);
        assert_eq!(history[2].version, 3);
    }

    #[test]
    fn test_multiple_models() {
        let reg = ModelRegistry::new();
        reg.register("gpt-4", "openai", serde_json::json!({}));
        reg.register("claude", "anthropic", serde_json::json!({}));
        let list = reg.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_missing_model() {
        let reg = ModelRegistry::new();
        assert!(reg.get_active("nonexistent").is_none());
        assert!(reg.promote("nonexistent", 1).is_err());
        assert!(reg.rollback("nonexistent").is_err());
        assert!(reg.deprecate("nonexistent", 1).is_err());
        assert!(reg.history("nonexistent").is_empty());
    }

    #[test]
    fn test_list_empty() {
        let reg = ModelRegistry::new();
        assert!(reg.list().is_empty());
    }

    #[test]
    fn test_version_format() {
        let formatted = version::format_version("gpt-4", 3);
        assert_eq!(formatted, "gpt-4@v3");
    }
}
