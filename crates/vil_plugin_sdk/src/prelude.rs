// =============================================================================
// VIL Plugin SDK Prelude — Single import for plugin authors
// =============================================================================
//
// Usage: use vil_plugin_sdk::prelude::*;

// Core trait
pub use crate::VilPlugin;

// Registration context
pub use crate::PluginContext;
pub use crate::ResourceRegistry;
pub use crate::PluginCapability;
pub use crate::EndpointSpec;
pub use crate::PluginDependency;
pub use crate::PluginHealth;

// Service building
pub use crate::ServiceProcess;
pub use crate::VxLane;

// Handler types
pub use crate::VilError;
pub use crate::VilResponse;
pub use crate::ServiceCtx;
pub use crate::ShmSlice;
pub use crate::Method;

// Routing
pub use crate::{get, post, put, delete};

// Serde (plugin config types need these)
pub use serde::{Serialize, Deserialize};

// Arc (plugins typically share state via Arc)
pub use std::sync::Arc;
